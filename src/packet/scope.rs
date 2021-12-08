use byteorder::{ReadBytesExt, WriteBytesExt};
use std::io::Read;

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error("invalid scope {:?} with extra {:?}", kind, extra)]
    InvalidScope { kind: u8, extra: u8 },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
type Result<T> = std::result::Result<T, ScopeError>;

// u16
// byte 0: scope_id: u8,
// byte 1: scope_extra: u8,
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// a single player
    Player(PlayerScope),

    /// all players in my map
    Map(MapScope),

    // all players in my server
    Server(ServerScope),
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
            Scope::Player(PlayerScope { player_id }) => {
                data.write_u8(*player_id)?;
            }

            Scope::Map(MapScope { have_plugin }) => {
                data.write_u8(if *have_plugin { 0b1000_0000 } else { 0 })?;
            }
            Scope::Server(ServerScope { have_plugin }) => {
                data.write_u8(if *have_plugin { 0b1000_0000 } else { 0 })?;
            }
        }

        Ok(data)
    }

    pub(crate) fn decode(data_stream: &mut impl Read) -> Result<Self> {
        let kind = data_stream.read_u8()?;
        let extra = data_stream.read_u8()?;

        let scope = match kind {
            0 => Scope::Player(PlayerScope { player_id: extra }),

            1 => {
                let have_plugin = (extra & 0b1000_0000) != 0;
                Scope::Map(MapScope { have_plugin })
            }

            2 => {
                let have_plugin = (extra & 0b1000_0000) != 0;
                Scope::Server(ServerScope { have_plugin })
            }

            _ => {
                return Err(ScopeError::InvalidScope { kind, extra });
            }
        };

        Ok(scope)
    }
}
impl From<PlayerScope> for Scope {
    fn from(scope: PlayerScope) -> Self {
        Self::Player(scope)
    }
}
impl From<MapScope> for Scope {
    fn from(scope: MapScope) -> Self {
        Self::Map(scope)
    }
}
impl From<ServerScope> for Scope {
    fn from(scope: ServerScope) -> Self {
        Self::Server(scope)
    }
}

/// a single player
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerScope {
    /// target player id if from client
    ///
    /// sender player id if from server
    pub player_id: u8,
}
impl PlayerScope {
    pub fn new(player_id: u8) -> Self {
        Self { player_id }
    }
}

/// all players in my map
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MapScope {
    // mask 1000_0000
    /// only send to those that have the same plugin that uses the same channel
    /// this was sent from
    pub have_plugin: bool,
}

// all players in my server
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ServerScope {
    // mask 1000_0000
    /// only send to those that have the same plugin that uses the same channel
    /// this was sent from
    pub have_plugin: bool,
}
