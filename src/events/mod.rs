pub mod store;

use self::store::Store;
use crate::{packet::Packet, RELAY_CHANNEL_START_INDEX};
use classicube_helpers::{events::net::PluginMessageReceivedEventHandler, tick};
use std::{
    cell::RefCell,
    io::{Cursor, Write},
    rc::Rc,
};
use tracing::error;

pub type CallbackFn = Box<dyn Fn(u8, &[u8])>;

#[derive(Debug, thiserror::Error)]
pub enum PartialStreamError {
    #[error("{0} < RELAY_CHANNEL_START_INDEX")]
    StartIndex(u8),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
type Result<T> = std::result::Result<T, PartialStreamError>;

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
    _plugin_message_handler: PluginMessageReceivedEventHandler,
    _tick_handler: tick::TickEventHandler,
}

impl RelayListener {
    pub fn new(channel: u8) -> Result<Self> {
        if channel < RELAY_CHANNEL_START_INDEX {
            return Err(PartialStreamError::StartIndex(channel));
        }

        let store: Rc<RefCell<Store>> = Default::default();

        let mut plugin_message_handler = PluginMessageReceivedEventHandler::new();
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
