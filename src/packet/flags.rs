use byteorder::{ReadBytesExt, WriteBytesExt};
use std::io::Read;

#[derive(Debug, thiserror::Error)]
pub enum FlagsError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
type Result<T> = std::result::Result<T, FlagsError>;

// u8
// is_packet_start: mask 1000_0000
// stream_id: mask 0111_1111
#[derive(Debug, PartialEq, Eq)]
pub struct Flags {
    /// is a start packet, or is a continuation
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
