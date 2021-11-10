use crate::{
    error::*,
    packet::{ContinuePacket, Packet, PlayerScope, Scope, StartPacket},
    RELAY_CHANNEL_START_INDEX,
};
use classicube_helpers::{events::plugin_messages, tick};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Cursor, Write},
    rc::Rc,
    time::{Duration, Instant},
};
use tracing::{debug, error, warn};

pub type CallbackFn = Box<dyn Fn(u8, &[u8])>;

#[derive(Debug)]
struct PartialStream {
    player_id: u8,
    data_length: u16,
    data_buffer: Vec<u8>,
}

impl PartialStream {
    pub fn is_finished(&self) -> bool {
        self.data_buffer.len() == self.data_length as usize
    }

    pub fn write_part(&mut self, data_part: Vec<u8>) -> Result<()> {
        let len = data_part
            .len()
            .min(self.data_length as usize - self.data_buffer.len());
        self.data_buffer.write_all(&data_part[..len])?;

        Ok(())
    }
}

pub struct RelayListener {
    pub channel: u8,
    store: Rc<RefCell<Store>>,
    _plugin_message_handler: plugin_messages::ReceivedEventHandler,
    _tick_handler: tick::TickEventHandler,
}

impl RelayListener {
    pub fn new(channel: u8) -> Result<Self> {
        ensure!(
            channel >= RELAY_CHANNEL_START_INDEX,
            "channel < RELAY_CHANNEL_START_INDEX"
        );

        let store: Rc<RefCell<Store>> = Default::default();

        let mut plugin_message_handler = plugin_messages::ReceivedEventHandler::new();
        {
            let store = Rc::downgrade(&store);
            plugin_message_handler.on(move |event| {
                if channel != event.channel {
                    return;
                }

                if let Some(store) = store.upgrade() {
                    let mut store = store.borrow_mut();
                    let store = &mut *store;
                    match Packet::decode(&mut Cursor::new(&event.data)) {
                        Ok(packet) => {
                            if let Err(e) = store.process_packet(packet) {
                                error!("processing packet: {:#?}", e);
                            }
                        }

                        Err(e) => {
                            error!("decoding packet: {:#?}", e);
                        }
                    }
                }
            });
        }
        let mut tick_handler = tick::TickEventHandler::new();
        {
            let store = Rc::downgrade(&store);
            tick_handler.on(move |_event| {
                if let Some(store) = store.upgrade() {
                    let mut store = store.borrow_mut();
                    let store = &mut *store;
                    store.tick();
                }
            });
        }

        Ok(Self {
            channel,
            store,
            _plugin_message_handler: plugin_message_handler,
            _tick_handler: tick_handler,
        })
    }

    pub fn on<F>(&mut self, callback: F)
    where
        F: Fn(u8, &[u8]),
        F: 'static,
    {
        let mut store = self.store.borrow_mut();
        store.event_handlers.push(Box::new(callback));
    }
}

#[derive(Default)]
pub struct Store {
    event_handlers: Vec<CallbackFn>,
    streams: HashMap<u8, PartialStream>,
    cleanup_times: HashMap<u8, Instant>,
}

impl Store {
    const STREAM_TIMEOUT: Duration = Duration::from_secs(10);

    fn process_packet(&mut self, packet: Packet) -> Result<()> {
        debug!("process_packet {:?}", packet);

        let finished_stream = match packet {
            Packet::Start(StartPacket {
                stream_id,
                scope,
                data_length,
                data_part,
            }) => {
                if let Scope::Player(PlayerScope { player_id }) = scope {
                    debug!(stream_id, player_id, data_length, "new stream");

                    let mut stream = PartialStream {
                        player_id,
                        data_length,
                        data_buffer: Vec::with_capacity(data_length as usize),
                    };
                    stream.write_part(data_part)?;

                    if let Some(old_stream) = self.streams.remove(&stream_id) {
                        warn!("restarting stream {:?}", old_stream);
                    }
                    if stream.is_finished() {
                        Some(stream)
                    } else {
                        self.streams.insert(stream_id, stream);
                        self.cleanup_times
                            .insert(stream_id, Instant::now() + Self::STREAM_TIMEOUT);
                        None
                    }
                } else {
                    bail!("got non-Player scope");
                }
            }

            Packet::Continue(ContinuePacket {
                stream_id,
                data_part,
            }) => {
                let is_finished = if let Some(stream) = self.streams.get_mut(&stream_id) {
                    stream.write_part(data_part)?;
                    debug!(
                        stream_id,
                        player_id = stream.player_id,
                        current_length = stream.data_buffer.len(),
                        data_length = stream.data_length,
                        "continue stream"
                    );

                    stream.is_finished()
                } else {
                    bail!("found continue packet before start");
                };

                if is_finished {
                    Some(self.streams.remove(&stream_id).unwrap())
                } else {
                    None
                }
            }
        };

        if let Some(stream) = finished_stream {
            debug!("finished_stream");
            for f in &self.event_handlers {
                f(stream.player_id, &stream.data_buffer);
            }
        }

        Ok(())
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let mut stream_ids_to_removes = self
            .cleanup_times
            .iter()
            .filter_map(|(stream_id, cleanup_time)| {
                if &now > cleanup_time {
                    Some(*stream_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for stream_id in stream_ids_to_removes.drain(..) {
            debug!(stream_id, "timed out, removing");
            self.cleanup_times.remove(&stream_id);
            self.streams.remove(&stream_id);
        }
    }
}
