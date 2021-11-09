use crate::error::*;
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use lazy_static::lazy_static;
use std::{
    collections::HashSet,
    io::{Cursor, Read, Write},
    sync::Mutex,
};

pub const PLUGIN_MESSAGE_DATA_LENGTH: usize = 64;

lazy_static! {
    static ref OUTGOING_PACKET_ID: Mutex<HashSet<u8>> = Default::default();
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

// [u8; 64]
#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    Start(StartPacket),
    Continue(ContinuePacket),
}
impl Packet {
    pub fn make_packets(data: &[u8], scope: Scope) -> Result<Vec<Self>> {
        ensure!(data.len() <= u16::MAX as usize, "data.len() > u16::MAX");
        let stream_id = new_outgoing_packet_id()?;

        let mut packets = vec![];

        let mut cursor = Cursor::new(data);
        packets.push(Packet::Start(StartPacket::new_reader(
            stream_id,
            scope,
            data.len() as u16,
            &mut cursor,
        )?));

        while cursor.position() < data.len() as u64 {
            packets.push(Packet::Continue(ContinuePacket::new_reader(
                stream_id,
                &mut cursor,
            )?));
        }

        Ok(packets)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        match self {
            Self::Start(packet) => packet.encode(),
            Self::Continue(packet) => packet.encode(),
        }
    }

    pub fn decode(data_stream: &mut impl Read) -> Result<Self> {
        let flags = Flags::decode(data_stream)?;

        let packet = if flags.is_packet_start {
            Packet::Start(StartPacket::decode(flags.stream_id, data_stream)?)
        } else {
            Packet::Continue(ContinuePacket::decode(flags.stream_id, data_stream)?)
        };

        Ok(packet)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct StartPacket {
    pub stream_id: u8,
    // this is always Player if sending from server
    pub scope: Scope,
    pub data_length: u16,
    // [u8; 64 - 2 * 2 - 1]
    pub data_part: Vec<u8>,
}
impl StartPacket {
    pub const DATA_PART_LENGTH: usize = 64 - 2 * 2 - 1;

    pub fn new(stream_id: u8, scope: Scope, data_length: u16, data_part: Vec<u8>) -> Result<Self> {
        ensure!(
            data_part.len() == Self::DATA_PART_LENGTH,
            "wrong data_part len"
        );

        Ok(Self {
            stream_id,
            scope,
            data_length,
            data_part,
        })
    }

    pub fn new_reader(
        stream_id: u8,
        scope: Scope,
        data_length: u16,
        data_stream: &mut impl Read,
    ) -> Result<Self> {
        let mut data_part = Vec::with_capacity(PLUGIN_MESSAGE_DATA_LENGTH);

        let mut buf = [0; Self::DATA_PART_LENGTH];
        let n = data_stream.read(&mut buf)?;

        data_part.write_all(&buf[..n])?;
        data_part.resize(Self::DATA_PART_LENGTH, 0);

        Self::new(stream_id, scope, data_length, data_part)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(PLUGIN_MESSAGE_DATA_LENGTH);

        data.write_all(
            &Flags {
                is_packet_start: true,
                stream_id: self.stream_id,
            }
            .encode()?,
        )?;
        data.write_all(&self.scope.encode()?)?;
        data.write_u16::<NetworkEndian>(self.data_length)?;
        data.write_all(&self.data_part)?;

        Ok(data)
    }

    fn decode(stream_id: u8, data_stream: &mut impl Read) -> Result<Self> {
        let scope = Scope::decode(data_stream)?;
        let data_length = data_stream.read_u16::<NetworkEndian>()?;

        let mut data_part = [0; Self::DATA_PART_LENGTH];
        let n = data_stream.read(&mut data_part)?;
        ensure!(n == Self::DATA_PART_LENGTH, "couldn't read full data_part");

        Ok(Self {
            stream_id,
            scope,
            data_length,
            data_part: data_part.to_vec(),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContinuePacket {
    pub stream_id: u8,
    pub data_part: Vec<u8>,
}
impl ContinuePacket {
    pub const DATA_PART_LENGTH: usize = 64 - 1;

    pub fn new(stream_id: u8, data_part: Vec<u8>) -> Result<Self> {
        ensure!(
            data_part.len() == Self::DATA_PART_LENGTH,
            "wrong data_part len"
        );

        Ok(Self {
            stream_id,
            data_part,
        })
    }

    pub fn new_reader(stream_id: u8, data_stream: &mut impl Read) -> Result<Self> {
        let mut data_part = Vec::with_capacity(PLUGIN_MESSAGE_DATA_LENGTH);

        let mut buf = [0; Self::DATA_PART_LENGTH];
        let n = data_stream.read(&mut buf)?;

        data_part.write_all(&buf[..n])?;
        data_part.resize(Self::DATA_PART_LENGTH, 0);

        Self::new(stream_id, data_part)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(PLUGIN_MESSAGE_DATA_LENGTH);

        data.write_all(
            &Flags {
                is_packet_start: false,
                stream_id: self.stream_id,
            }
            .encode()?,
        )?;
        data.write_all(&self.data_part)?;

        Ok(data)
    }

    fn decode(stream_id: u8, data_stream: &mut impl Read) -> Result<Self> {
        let mut data_part = [0; Self::DATA_PART_LENGTH];
        let n = data_stream.read(&mut data_part)?;
        ensure!(n == Self::DATA_PART_LENGTH, "couldn't read full data_part");

        Ok(Self {
            stream_id,
            data_part: data_part.to_vec(),
        })
    }
}

// u8
// is_packet_start: mask 1000_0000
// stream_id: mask 0111_1111
#[derive(Debug, PartialEq, Eq)]
pub struct Flags {
    // is a start packet, or is a continuation
    pub is_packet_start: bool,

    // TODO what am i
    pub stream_id: u8,
}
impl Flags {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(1);

        let mut b = 0;
        b |= if self.is_packet_start { 0b1000_0000 } else { 0 };
        b |= self.stream_id & 0b0111_1111;

        data.write_u8(b)?;

        Ok(data)
    }

    pub fn decode(data_stream: &mut impl Read) -> Result<Self> {
        let byte = data_stream.read_u8()?;
        let is_packet_start = (byte & 0b1000_0000) != 0;
        let stream_id = byte & 0b0111_1111;
        Ok(Self {
            is_packet_start,
            stream_id,
        })
    }
}

// u16
// byte 0: scope_id: u8,
// byte 1: scope_extra: u8,
#[derive(Debug, PartialEq, Eq)]
pub enum Scope {
    // a single player
    Player {
        // target player id if from client
        // sender player id if from server
        player_id: u8,
    },

    // all players in my map
    Map {
        // mask 1000_0000
        // only send to those that have the same plugin that uses the same channel
        // this was sent from
        have_plugin: bool,
    },

    // all players in my server
    Server {
        // mask 1000_0000
        have_plugin: bool,
    },
}
impl Scope {
    pub fn kind(&self) -> u8 {
        match self {
            Scope::Player { .. } => 0,
            Scope::Map { .. } => 1,
            Scope::Server { .. } => 2,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(2);

        data.write_u8(self.kind())?;

        match self {
            Scope::Player { player_id } => {
                data.write_u8(*player_id)?;
            }

            Scope::Map { have_plugin } => {
                data.write_u8(if *have_plugin { 0b1000_0000 } else { 0 })?;
            }
            Scope::Server { have_plugin } => {
                data.write_u8(if *have_plugin { 0b1000_0000 } else { 0 })?;
            }
        }

        Ok(data)
    }

    fn decode(data_stream: &mut impl Read) -> Result<Self> {
        let kind = data_stream.read_u8()?;
        let extra = data_stream.read_u8()?;

        let scope = match kind {
            0 => Scope::Player { player_id: extra },

            1 => {
                let have_plugin = (extra & 0b1000_0000) != 0;
                Scope::Map { have_plugin }
            }

            2 => {
                let have_plugin = (extra & 0b1000_0000) != 0;
                Scope::Server { have_plugin }
            }

            _ => {
                bail!("invalid scope {:?} with extra {:?}", kind, extra);
            }
        };

        Ok(scope)
    }
}
