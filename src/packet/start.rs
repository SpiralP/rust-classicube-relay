use super::{flags::Flags, scope::Scope, FlagsError, Packet, ScopeError};
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

#[derive(Debug, thiserror::Error)]
pub enum StartPacketError {
    #[error("wrong data_part len")]
    DataPartLength,

    #[error("couldn't read full data_part")]
    ReadFullDataPart,

    #[error(transparent)]
    Flags(#[from] FlagsError),

    #[error(transparent)]
    Scope(#[from] ScopeError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
type Result<T> = std::result::Result<T, StartPacketError>;

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

    pub fn new<S: Into<Scope>>(
        stream_id: u8,
        scope: S,
        data_length: u16,
        data_part: Vec<u8>,
    ) -> Result<Self> {
        if data_part.len() != Self::DATA_PART_LENGTH {
            return Err(StartPacketError::DataPartLength);
        }

        Ok(Self {
            stream_id,
            scope: scope.into(),
            data_length,
            data_part,
        })
    }

    pub fn new_reader<S: Into<Scope>>(
        stream_id: u8,
        scope: S,
        data_length: u16,
        data_stream: &mut impl Read,
    ) -> Result<Self> {
        let mut data_part = Vec::with_capacity(Packet::DATA_LENGTH);

        let mut buf = [0; Self::DATA_PART_LENGTH];
        let n = data_stream.read(&mut buf)?;

        data_part.write_all(&buf[..n])?;
        data_part.resize(Self::DATA_PART_LENGTH, 0);

        Self::new(stream_id, scope, data_length, data_part)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(Packet::DATA_LENGTH);

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

    pub(crate) fn decode(stream_id: u8, data_stream: &mut impl Read) -> Result<Self> {
        let scope = Scope::decode(data_stream)?;
        let data_length = data_stream.read_u16::<NetworkEndian>()?;

        let mut data_part = [0; Self::DATA_PART_LENGTH];
        let n = data_stream.read(&mut data_part)?;

        if n != Self::DATA_PART_LENGTH {
            return Err(StartPacketError::ReadFullDataPart);
        }

        Ok(Self {
            stream_id,
            scope,
            data_length,
            data_part: data_part.to_vec(),
        })
    }
}
