use crate::{
    packet::{ContinuePacket, ContinuePacketError, Scope, StartPacket, StartPacketError},
    Packet,
};
use lazy_static::lazy_static;
use std::{collections::HashSet, io::Cursor, sync::Mutex};

pub const PLUGIN_MESSAGE_DATA_LENGTH: usize = 64;

lazy_static! {
    static ref OUTGOING_PACKET_ID: Mutex<HashSet<u8>> = Default::default();
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("{0}")]
    LengthOverflow(String),

    #[error("can't find free outgoing packet id")]
    PacketIdLimit,

    #[error(transparent)]
    StartPacket(#[from] StartPacketError),

    #[error(transparent)]
    ContinuePacket(#[from] ContinuePacketError),
}
type Result<T> = std::result::Result<T, StreamError>;

// [u8; 64]
#[derive(Debug)]
pub struct Stream {
    stream_id: u8,
    pub data: Vec<u8>,
    pub scope: Scope,
}
impl Stream {
    pub fn new<S: Into<Scope>>(data: Vec<u8>, scope: S) -> Result<Self> {
        if data.len() > u16::MAX as usize {
            return Err(StreamError::LengthOverflow(
                "data.len() > u16::MAX".to_string(),
            ));
        }

        let stream_id = Self::new_outgoing_packet_id()?;
        Ok(Self {
            stream_id,
            data,
            scope: scope.into(),
        })
    }

    pub fn packets(&self) -> Result<Vec<Packet>> {
        if self.data.len() > u16::MAX as usize {
            return Err(StreamError::LengthOverflow(
                "data.len() > u16::MAX".to_string(),
            ));
        }

        let mut packets = vec![];

        let mut cursor = Cursor::new(&self.data);
        packets.push(Packet::Start(StartPacket::new_reader(
            self.stream_id,
            self.scope.clone(),
            self.data.len() as u16,
            &mut cursor,
        )?));

        while cursor.position() < self.data.len() as u64 {
            packets.push(Packet::Continue(ContinuePacket::new_reader(
                self.stream_id,
                &mut cursor,
            )?));
        }

        Ok(packets)
    }

    fn new_outgoing_packet_id() -> Result<u8> {
        let mut guard = OUTGOING_PACKET_ID.lock().unwrap();

        let maybe_id = (0..2u8.pow(7)).find(|id| !guard.contains(id));
        if let Some(id) = maybe_id {
            guard.insert(id);
            Ok(id)
        } else {
            Err(StreamError::PacketIdLimit)
        }
    }

    fn free_outgoing_packet_id(id: u8) {
        let mut guard = OUTGOING_PACKET_ID.lock().unwrap();
        guard.remove(&id);
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        Self::free_outgoing_packet_id(self.stream_id);
    }
}
