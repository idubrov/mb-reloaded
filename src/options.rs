use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Read;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WinCondition {
  ByWins,
  ByMoney,
}

#[derive(Debug)]
pub struct Options {
  pub players: u8,
  pub treasures: u8,
  pub rounds: u16,
  pub cash: u16,
  pub round_time: Duration,
  // Each point is 3% slowdown from 100%
  // 0 is 100%
  // 33 is 1%
  pub speed: u16,
  pub darkness: bool,
  pub free_market: bool,
  pub selling: bool,
  pub win: WinCondition,
  pub bomb_damage: u8,
  pub campaign_mode: bool,
}

impl Default for Options {
  fn default() -> Self {
    Options {
      players: 2,
      treasures: 45,
      rounds: 15,
      cash: 750,
      round_time: Duration::from_secs(420),
      speed: 8,
      darkness: false,
      free_market: false,
      selling: false,
      win: WinCondition::ByMoney,
      bomb_damage: 100,
      campaign_mode: false,
    }
  }
}

impl Options {
  fn from_binary(buf: &[u8]) -> Self {
    // Invalid options file; just use defaults
    if buf.len() != 17 {
      return Default::default();
    }

    let mut it = buf;
    let mut opts = Options {
      players: it.read_u8().unwrap(),
      treasures: it.read_u8().unwrap(),
      rounds: it.read_u16::<LittleEndian>().unwrap(),
      cash: it.read_u16::<LittleEndian>().unwrap(),
      round_time: to_duration(it.read_u32::<LittleEndian>().unwrap()),
      speed: it.read_u16::<LittleEndian>().unwrap(),
      darkness: it.read_u8().unwrap() != 0,
      free_market: it.read_u8().unwrap() != 0,
      selling: it.read_u8().unwrap() != 0,
      win: if it.read_u8().unwrap() != 0 {
        WinCondition::ByWins
      } else {
        WinCondition::ByMoney
      },
      bomb_damage: it.read_u8().unwrap(),
      campaign_mode: false,
    };
    if opts.players > 4 {
      opts.players = 2;
    }
    if opts.bomb_damage > 100 {
      opts.players = 100;
    }
    if opts.rounds > 55 {
      opts.rounds = 55;
    }
    if opts.treasures > 75 {
      opts.treasures = 75;
    }
    if opts.cash > 2650 {
      opts.cash = 2650;
    }
    if opts.speed > 33 {
      opts.speed = 33;
    }
    opts
  }

  /// Load options from a configuration file. This function uses the same format as the original game.
  pub fn load(game_dir: &Path) -> Self {
    let path = game_dir.join("OPTIONS.CFG");
    let mut buf: [u8; 17] = [0; 17];
    std::fs::File::open(path)
      .and_then(|mut file| file.read_exact(&mut buf))
      .map(|()| Options::from_binary(&buf))
      .unwrap_or_default()
  }

  /// Save options into a binary slice
  pub fn save(&self, game_dir: &Path) -> Result<(), anyhow::Error> {
    let data = self.save_inner();
    let path = game_dir.join("OPTIONS.CFG");
    // FIXME: either proper errors or logging
    std::fs::write(path, data)?;
    Ok(())
  }

  /// Save options into a binary slice
  fn save_inner(&self) -> Vec<u8> {
    let mut buf = Vec::with_capacity(17);
    buf.write_u8(self.players).unwrap();
    buf.write_u8(self.treasures).unwrap();
    buf.write_u16::<LittleEndian>(self.rounds).unwrap();
    buf.write_u16::<LittleEndian>(self.cash).unwrap();
    buf.write_u32::<LittleEndian>(from_duration(self.round_time)).unwrap();
    buf.write_u16::<LittleEndian>(self.speed).unwrap();
    buf.write_u8(self.darkness as u8).unwrap();
    buf.write_u8(self.free_market as u8).unwrap();
    buf.write_u8(self.selling as u8).unwrap();
    if self.win == WinCondition::ByWins {
      buf.write_u8(1).unwrap();
    } else {
      buf.write_u8(0).unwrap();
    };
    buf.write_u8(self.bomb_damage).unwrap();
    assert_eq!(buf.len(), 17);
    buf
  }
}

/// Convert internal representation of time proper duration. 18.2 interrupts per second was standard
/// CMOS realtime clock interrupt frequency.
fn to_duration(value: u32) -> Duration {
  let seconds = (value as u64) * 10 / 182;
  Duration::from_secs(seconds)
}

/// Convert internal representation of time proper duration. 18.2 interrupts per second was standard
/// CMOS realtime clock interrupt frequency.
fn from_duration(value: Duration) -> u32 {
  (value.as_secs() * 182 / 10) as u32
}
