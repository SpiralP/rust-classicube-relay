use super::{flags::Flags, FlagsError, Packet};
use std::io::{Read, Write};

#[derive(Debug, thiserror::Error)]
pub enum ContinuePacketError {
    #[error("wrong data_part len")]
    DataPartLength,

    #[error("couldn't read full data_part")]
    ReadFullDataPart,

    #[error(transparent)]
    Flags(#[from] FlagsError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
type Result<T> = std::result::Result<T, ContinuePacketError>;

#[derive(Debug, PartialEq, Eq)]
pub struct ContinuePacket {
    pub stream_id: u8,
    pub data_part: Vec<u8>,
}
impl ContinuePacket {
    pub const DATA_PART_LENGTH: usize = 64 - 1;

    pub fn new(stream_id: u8, data_part: Vec<u8>) -> Result<Self> {
        if data_part.len() != Self::DATA_PART_LENGTH {
            return Err(ContinuePacketError::DataPartLength);
        }

        Ok(Self {
            stream_id,
            data_part,
        })
    }

    pub fn new_reader(stream_id: u8, data_stream: &mut impl Read) -> Result<Self> {
        let mut data_part = Vec::with_capacity(Packet::DATA_LENGTH);

        let mut buf = [0; Self::DATA_PART_LENGTH];
        let n = data_stream.read(&mut buf)?;

        data_part.write_all(&buf[..n])?;
        data_part.resize(Self::DATA_PART_LENGTH, 0);

        Self::new(stream_id, data_part)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(Packet::DATA_LENGTH);

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

    pub(crate) fn decode(stream_id: u8, data_stream: &mut impl Read) -> Result<Self> {
        let mut data_part = [0; Self::DATA_PART_LENGTH];
        let n = data_stream.read(&mut data_part)?;

        if n != Self::DATA_PART_LENGTH {
            return Err(ContinuePacketError::ReadFullDataPart);
        }

        Ok(Self {
            stream_id,
            data_part: data_part.to_vec(),
        })
    }
}
