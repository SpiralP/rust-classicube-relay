#![doc = include_str!("../README.md")]

pub mod events;
pub mod packet;
pub mod stream;
#[cfg(test)]
mod tests;

pub use self::{events::RelayListener, packet::Packet, stream::Stream};

/// start index of channels that the relay plugin listen for
pub const RELAY_CHANNEL_START_INDEX: u8 = 200;
