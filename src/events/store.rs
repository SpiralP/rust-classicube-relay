use super::{CallbackFn, PartialStream, PartialStreamError};
use crate::packet::{ContinuePacket, Packet, PlayerScope, Scope, StartPacket};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tracing::{debug, error, warn};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("got non-Player scope")]
    Thing,

    #[error("found continue packet before start")]
    Thing2,

    #[error(transparent)]
    PartialStream(#[from] PartialStreamError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
type Result<T> = std::result::Result<T, StoreError>;

#[derive(Default)]
pub(crate) struct Store {
    pub(crate) event_handlers: Vec<CallbackFn>,
    streams: HashMap<u8, PartialStream>,
    cleanup_times: HashMap<u8, Instant>,
}

impl Store {
    const STREAM_TIMEOUT: Duration = Duration::from_secs(10);

    pub(crate) fn process_packet(&mut self, packet: Packet) -> Result<()> {
        debug!("process_packet {:?}", packet);

        let finished_stream = match packet {
            Packet::Start(StartPacket {
                stream_id,
                scope,
                data_length,
                data_part,
            }) => {
                if let Scope::Player(PlayerScope { player_id }) = scope {
                    debug!(stream_id, player_id, data_length, "new stream");

                    let mut stream = PartialStream {
                        player_id,
                        data_length,
                        data_buffer: Vec::with_capacity(data_length as usize),
                    };
                    stream.write_part(data_part)?;

                    if let Some(old_stream) = self.streams.remove(&stream_id) {
                        warn!("restarting stream {:?}", old_stream);
                    }
                    if stream.is_finished() {
                        Some(stream)
                    } else {
                        self.streams.insert(stream_id, stream);
                        self.cleanup_times
                            .insert(stream_id, Instant::now() + Self::STREAM_TIMEOUT);
                        None
                    }
                } else {
                    return Err(StoreError::Thing);
                }
            }

            Packet::Continue(ContinuePacket {
                stream_id,
                data_part,
            }) => {
                let is_finished = if let Some(stream) = self.streams.get_mut(&stream_id) {
                    stream.write_part(data_part)?;
                    debug!(
                        stream_id,
                        player_id = stream.player_id,
                        current_length = stream.data_buffer.len(),
                        data_length = stream.data_length,
                        "continue stream"
                    );

                    stream.is_finished()
                } else {
                    return Err(StoreError::Thing2);
                };

                if is_finished {
                    self.cleanup_times.remove(&stream_id);
                    Some(self.streams.remove(&stream_id).unwrap())
                } else {
                    None
                }
            }
        };

        if let Some(stream) = finished_stream {
            debug!("finished_stream");
            for f in &self.event_handlers {
                f(stream.player_id, &stream.data_buffer);
            }
        }

        Ok(())
    }

    pub(crate) fn tick(&mut self) {
        let now = Instant::now();
        let mut stream_ids_to_removes = self
            .cleanup_times
            .iter()
            .filter_map(|(stream_id, cleanup_time)| {
                if &now > cleanup_time {
                    Some(*stream_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for stream_id in stream_ids_to_removes.drain(..) {
            debug!(stream_id, "timed out, removing");
            self.cleanup_times.remove(&stream_id);
            self.streams.remove(&stream_id);
        }
    }
}
