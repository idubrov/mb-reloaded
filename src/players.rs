//! Player statistics
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Failed to load player statistics from '{path}'")]
pub struct StatsError {
    #[source]
    source: anyhow::Error,
    path: PathBuf,
}

#[derive(Default)]
pub struct PlayerStats {
    pub name: String,
}

#[derive(Default)]
pub struct Players {
    pub players: Box<[PlayerStats; 32]>,
}

impl Players {
    /// Load player statistics from `PLAYERS.DAT` file.
    pub fn load_players(game_dir: &Path) -> Result<Players, StatsError> {
        let path = game_dir.join("PLAYERS.DAT");
        Players::load_players_internal(&path).map_err(|source| StatsError { path, source })
    }

    fn load_players_internal(path: &Path) -> Result<Players, anyhow::Error> {
        let data = std::fs::read(path)?;
        let mut players = Players::default();
        // Invalid format, just ignore
        if data.len() != 3232 {
            return Ok(players);
        }

        for player in 0..32 {
            // Each record is 101 byte long
            let data = &data[player * 101..][..101];
            // `1` indicates an empty record.
            if data[0] != 1 {
                let len = usize::from(data[1].min(26));
                players.players[player].name =
                    String::from_utf8_lossy(&data[2..2 + len]).into_owned();
            }
        }

        Ok(players)
    }
}
