//! Player statistics
use byteorder::{LittleEndian, ReadBytesExt};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Failed to load player statistics from '{path}'")]
pub struct PlayersLoadError {
  #[source]
  source: std::io::Error,
  path: PathBuf,
}

#[derive(Debug, Error)]
#[error("Failed to save player statistics to '{path}'")]
pub struct PlayersSaveError {
  #[source]
  source: std::io::Error,
  path: PathBuf,
}

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

impl Default for PlayerStats {
  fn default() -> Self {
    Self {
      name: String::new(),
      tournaments: 0,
      tournaments_wins: 0,
      rounds: 0,
      rounds_wins: 0,
      treasures_collected: 0,
      total_money: 0,
      bombs_bought: 0,
      bombs_dropped: 0,
      deaths: 0,
      meters_ran: 0,
      history: vec![0; 34],
    }
  }
}

#[derive(Default)]
pub struct PlayersRoster {
  pub players: Box<[Option<PlayerStats>; 32]>,
}

impl PlayersRoster {
  /// Load player statistics from `PLAYERS.DAT` file.
  pub fn load(game_dir: &Path) -> Result<PlayersRoster, PlayersLoadError> {
    let path = game_dir.join("PLAYERS.DAT");
    PlayersRoster::load_players_internal(&path).map_err(|source| PlayersLoadError { path, source })
  }

  fn load_players_internal(path: &Path) -> Result<PlayersRoster, std::io::Error> {
    let data = std::fs::read(path)?;
    let mut players = PlayersRoster::default();
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

  pub fn save(&self, game_dir: &Path) -> Result<(), PlayersSaveError> {
    let mut out: Vec<u8> = Vec::with_capacity(32 * 101);
    for player in self.players.iter() {
      if let Some(record) = player {
        out.push(0);

        let name_len = record.name.len().min(24);
        out.push(name_len as u8);
        out.extend_from_slice(&record.name.as_bytes()[..name_len]);
        out.resize(out.len() + (24 - name_len), 0);

        for value in &[
          record.tournaments,
          record.tournaments_wins,
          record.rounds,
          record.rounds_wins,
          record.treasures_collected,
          record.total_money,
          record.bombs_bought,
          record.bombs_dropped,
          record.deaths,
          record.meters_ran,
        ] {
          out.extend_from_slice(&value.to_le_bytes());
        }

        out.extend_from_slice(&record.history);
        // FIXME: should this be history?
        out.push(0);
      } else {
        out.push(1);
        out.resize(out.len() + 100, 0);
      }
    }
    assert_eq!(32 * 101, out.len());

    let path = game_dir.join("PLAYERS.DAT");
    std::fs::write(&path, &out).map_err(|source| PlayersSaveError { path, source })?;
    Ok(())
  }
}
