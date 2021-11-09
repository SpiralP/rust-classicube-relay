use crate::{
    error::*,
    packet::{ContinuePacket, Scope, StartPacket},
    Packet,
};
use lazy_static::lazy_static;
use std::{collections::HashSet, io::Cursor, sync::Mutex};

pub const PLUGIN_MESSAGE_DATA_LENGTH: usize = 64;

lazy_static! {
    static ref OUTGOING_PACKET_ID: Mutex<HashSet<u8>> = Default::default();
}

// [u8; 64]
#[derive(Debug)]
pub struct Stream {
    stream_id: u8,
    pub data: Vec<u8>,
    pub scope: Scope,
}
impl Stream {
    pub fn new<S: Into<Scope>>(data: Vec<u8>, scope: S) -> Result<Self> {
        ensure!(data.len() <= u16::MAX as usize, "data.len() > u16::MAX");

        let stream_id = Self::new_outgoing_packet_id()?;
        Ok(Self {
            stream_id,
            data,
            scope: scope.into(),
        })
    }

    pub fn packets(&self) -> Result<Vec<Packet>> {
        ensure!(
            self.data.len() <= u16::MAX as usize,
            "data.len() > u16::MAX"
        );

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
            bail!("can't find free outgoing packet id");
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
