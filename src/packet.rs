use crate::error::*;
use byteorder::{NetworkEndian, WriteBytesExt};
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
#[derive(Debug)]
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

    pub fn decode(data: &[u8]) -> Result<Self> {
        ensure!(data.len() == PLUGIN_MESSAGE_DATA_LENGTH, "wrong data len");

        todo!();
    }
}

#[derive(Debug)]
pub struct StartPacket {
    pub flags: Flags,
    // this is always Player if sending from server
    pub scope: Scope,
    pub data_length: u16,
    // [u8; 64 - 2 * 2 - 1]
    pub data_part: Vec<u8>,
}
impl StartPacket {
    const DATA_PART_LENGTH: usize = 64 - 2 * 2 - 1;

    pub fn new(stream_id: u8, scope: Scope, data_length: u16, data_part: Vec<u8>) -> Result<Self> {
        ensure!(
            data_part.len() == Self::DATA_PART_LENGTH,
            "wrong data_part len"
        );

        Ok(Self {
            flags: Flags {
                is_packet_start: true,
                stream_id,
            },
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

        Ok(Self::new(stream_id, scope, data_length, data_part)?)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(PLUGIN_MESSAGE_DATA_LENGTH);

        data.write_all(&self.flags.encode()?)?;
        data.write_all(&self.scope.encode()?)?;
        data.write_u16::<NetworkEndian>(self.data_length)?;
        data.write_all(&self.data_part)?;

        Ok(data)
    }
}

#[derive(Debug)]
pub struct ContinuePacket {
    pub flags: Flags,
    pub data_part: Vec<u8>,
}
impl ContinuePacket {
    const DATA_PART_LENGTH: usize = 64 - 1;

    pub fn new(stream_id: u8, data_part: Vec<u8>) -> Result<Self> {
        ensure!(
            data_part.len() == Self::DATA_PART_LENGTH,
            "wrong data_part len"
        );

        Ok(Self {
            flags: Flags {
                is_packet_start: false,
                stream_id,
            },
            data_part,
        })
    }

    pub fn new_reader(stream_id: u8, data_stream: &mut impl Read) -> Result<Self> {
        let mut data_part = Vec::with_capacity(PLUGIN_MESSAGE_DATA_LENGTH);

        let mut buf = [0; Self::DATA_PART_LENGTH];
        let n = data_stream.read(&mut buf)?;

        data_part.write_all(&buf[..n])?;
        data_part.resize(Self::DATA_PART_LENGTH, 0);

        Ok(Self::new(stream_id, data_part)?)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(PLUGIN_MESSAGE_DATA_LENGTH);

        data.write_all(&self.flags.encode()?)?;
        data.write_all(&self.data_part)?;

        Ok(data)
    }
}

// u8
// is_packet_start: mask 1000_0000
// stream_id: mask 0111_1111
#[derive(Debug)]
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
}

// u16
// byte 0: scope_id: u8,
// byte 1: scope_extra: u8,
#[derive(Debug)]
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
    pub fn id(&self) -> u8 {
        match self {
            Scope::Player { .. } => 0,
            Scope::Map { .. } => 1,
            Scope::Server { .. } => 2,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(2);

        data.write_u8(self.id())?;

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
}

#[test]
fn test_plugin_messages_make_packets() {
    // test 0 length
    let mut packets = Packet::make_packets(&vec![], Scope::Player { player_id: 0 }).unwrap();
    assert_eq!(packets.len(), 1);
    if let Packet::Start(StartPacket {
        flags,
        scope: _scope,
        data_length,
        data_part,
    }) = packets.remove(0)
    {
        assert_eq!(flags.is_packet_start, true);
        assert_eq!(flags.stream_id, 0);
        assert_eq!(data_length, 0);
        assert_eq!(data_part, vec![0; 59]);
    } else {
        unreachable!();
    }

    // test 1 single start packet
    let mut packets = Packet::make_packets(b"helloooo", Scope::Player { player_id: 0 }).unwrap();
    assert_eq!(packets.len(), 1);
    if let Packet::Start(StartPacket {
        flags,
        scope: _scope,
        data_length,
        data_part,
    }) = packets.remove(0)
    {
        assert_eq!(flags.is_packet_start, true);
        assert_eq!(flags.stream_id, 1);
        assert_eq!(data_length, 8);
        let mut v = b"helloooo".to_vec();
        v.resize(59, 0);
        assert_eq!(data_part, v);
    } else {
        unreachable!();
    }

    // test multiple packets
    let mut packets = Packet::make_packets(&vec![123; 64], Scope::Player { player_id: 0 }).unwrap();
    assert_eq!(packets.len(), 2);
    if let Packet::Start(StartPacket {
        flags,
        scope: _scope,
        data_length,
        data_part,
    }) = packets.remove(0)
    {
        assert_eq!(flags.is_packet_start, true);
        assert_eq!(flags.stream_id, 2);
        assert_eq!(data_length, 64);
        assert_eq!(data_part, vec![123; 59]);
    } else {
        unreachable!();
    }
    if let Packet::Continue(ContinuePacket { flags, data_part }) = packets.remove(0) {
        assert_eq!(flags.is_packet_start, false);
        assert_eq!(flags.stream_id, 2);
        let mut v = vec![123; 5];
        v.resize(63, 0);
        assert_eq!(data_part, v);
    } else {
        unreachable!();
    }

    // test "no free outgoing ids"
    for _ in 0..128 - 3 {
        Packet::make_packets(b"", Scope::Player { player_id: 0 }).unwrap();
    }
    assert!(Packet::make_packets(b"", Scope::Player { player_id: 0 }).is_err());
}
