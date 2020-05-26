//! Manage which players were selected in the previous game
use std::path::Path;

#[derive(Debug, Default)]
pub struct Identities {
  /// Each value is the index in the players file. Up to 31 (inclusive), as we only support 32 players.
  pub players: [Option<u8>; 4],
}

impl Identities {
  /// Load player statistics from `PLAYERS.DAT` file.
  pub fn load_identities(game_dir: &Path) -> Identities {
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
}
