use crate::{
    error::*,
    packet::{ContinuePacket, Packet, PlayerScope, Scope, StartPacket},
    RELAY_CHANNEL_START_INDEX,
};
use classicube_helpers::events::plugin_messages;
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Cursor, Write},
    rc::Rc,
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

#[derive(Default)]
pub struct Store {
    event_handlers: Vec<CallbackFn>,
    streams: HashMap<u8, PartialStream>,
}

pub struct RelayListener {
    pub channel: u8,
    store: Rc<RefCell<Store>>,
    _plugin_message_handler: plugin_messages::ReceivedEventHandler,
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
                    match Packet::decode(&mut Cursor::new(&event.data)) {
                        Ok(packet) => {
                            if let Err(e) = Self::process_packet(packet, store) {
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

        Ok(Self {
            channel,
            store,
            _plugin_message_handler: plugin_message_handler,
        })
    }

    fn process_packet(packet: Packet, store: Rc<RefCell<Store>>) -> Result<()> {
        debug!("process_packet {:?}", packet);

        let mut guard = store.borrow_mut();
        let Store {
            streams,
            event_handlers,
        } = &mut *guard;

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

                    if let Some(old_stream) = streams.remove(&stream_id) {
                        warn!("restarting stream {:?}", old_stream);
                    }
                    if stream.is_finished() {
                        Some(stream)
                    } else {
                        streams.insert(stream_id, stream);
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
                let is_finished = if let Some(stream) = streams.get_mut(&stream_id) {
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
                    Some(streams.remove(&stream_id).unwrap())
                } else {
                    None
                }
            }
        };

        if let Some(stream) = finished_stream {
            debug!("finished_stream");
            for f in event_handlers {
                f(stream.player_id, &stream.data_buffer);
            }
        }

        Ok(())
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
