//! Player statistics
use byteorder::{LittleEndian, ReadBytesExt};
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
  pub tournaments: u32,
  pub tournaments_wins: u32,
  pub rounds: u32,
  pub rounds_wins: u32,
  pub treasures_collected: u32,
  pub total_money: u32,
  pub bombs_bought: u32,
  pub bombs_dropped: u32,
  pub deaths: u32,
  pub meters_ran: u32,
  pub history: Vec<u8>,
}

#[derive(Default)]
pub struct Players {
  pub players: Box<[Option<PlayerStats>; 32]>,
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

      // `0` indicates an active record (non-zero is an empty record).
      if data[0] == 0 {
        let record = &mut players.players[player].get_or_insert_with(Default::default);

        let len = usize::from(data[1].min(24));
        record.name = String::from_utf8_lossy(&data[2..2 + len]).into_owned();

        let mut it = &data[26..66];
        for ptr in &mut [
          &mut record.tournaments,
          &mut record.tournaments_wins,
          &mut record.rounds,
          &mut record.rounds_wins,
          &mut record.treasures_collected,
          &mut record.total_money,
          &mut record.bombs_bought,
          &mut record.bombs_dropped,
          &mut record.deaths,
          &mut record.meters_ran,
        ] {
          **ptr = it.read_u32::<LittleEndian>().unwrap();
        }
        record.history = data[66..][..34].to_vec();
      }
    }

    Ok(players)
  }
}
