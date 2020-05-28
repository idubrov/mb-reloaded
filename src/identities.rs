//! Manage which players were selected in the previous game
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Failed to save selected players at '{path}'")]
pub struct IdentitiesSaveError {
  path: PathBuf,
  #[source]
  source: std::io::Error,
}

#[derive(Debug, Default)]
pub struct Identities {
  /// Each value is the index in the players file. Up to 31 (inclusive), as we only support 32 players.
  pub players: [Option<u8>; 4],
}

impl Identities {
  /// Load players selected in the last game
  pub fn load(game_dir: &Path) -> Identities {
    let path = game_dir.join("IDENTIFY.DAT");
    match std::fs::read(path) {
      Ok(data) if data.len() == 4 => {
        let mut identities = Identities::default();
        for (idx, player_idx) in data.iter().enumerate() {
          if *player_idx != 0 {
            identities.players[idx] = Some((*player_idx - 1).min(31));
          }
        }
        identities
      }
      _ => Identities::default(),
    }
  }

  /// Save selected players
  pub fn save(&self, game_dir: &Path) -> Result<(), IdentitiesSaveError> {
    let path = game_dir.join("IDENTIFY.DAT");
    let mut output: [u8; 4] = [0; 4];
    for (idx, value) in self.players.iter().enumerate() {
      output[idx] = match value {
        None => 0,
        Some(value) => value + 1,
      }
    }
    std::fs::write(&path, &output).map_err(|source| IdentitiesSaveError { path, source })
  }
}
