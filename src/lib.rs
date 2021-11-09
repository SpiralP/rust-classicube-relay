#![doc = include_str!("../README.md")]

pub mod error;
pub mod events;
pub mod packet;

pub use self::{events::RelayListener, packet::Packet};

pub const RELAY_CHANNEL_START_INDEX: u8 = 200;
