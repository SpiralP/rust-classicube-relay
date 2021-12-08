mod r#continue;
mod flags;
mod scope;
mod start;

pub use self::{flags::*, r#continue::*, scope::*, start::*};
use std::io::Read;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("{0}")]
    LengthOverflow(String),

    #[error("can't find free outgoing packet id")]
    PacketIdLimit,

    #[error(transparent)]
    Flags(#[from] FlagsError),

    #[error(transparent)]
    StartPacket(#[from] StartPacketError),

    #[error(transparent)]
    ContinuePacket(#[from] ContinuePacketError),
}
type Result<T> = std::result::Result<T, StreamError>;

// [u8; 64]
#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    Start(StartPacket),
    Continue(ContinuePacket),
}
impl Packet {
    pub const DATA_LENGTH: usize = 64;

    pub fn encode(&self) -> Result<Vec<u8>> {
        match self {
            Self::Start(packet) => Ok(packet.encode()?),
            Self::Continue(packet) => Ok(packet.encode()?),
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
