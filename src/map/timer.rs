use crate::map::{LevelMap, MapValue, MAP_COLS, MAP_ROWS};
use rand::prelude::*;

/// Map for tracking timer counters.
pub struct TimerMap {
  data: Vec<u16>,
}

impl std::ops::Index<usize> for TimerMap {
  type Output = [u16];

  fn index(&self, row: usize) -> &[u16] {
    &self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

impl std::ops::IndexMut<usize> for TimerMap {
  fn index_mut(&mut self, row: usize) -> &mut [u16] {
    &mut self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

impl TimerMap {
  pub fn from_level_map(level_map: &LevelMap) -> Self {
    let mut rng = rand::thread_rng();
    let mut map = Self {
      data: vec![0; MAP_COLS * MAP_ROWS],
    };
    for row in 0..MAP_ROWS {
      for col in 0..MAP_COLS {
        if level_map[row][col] == MapValue::BioMass {
          map[row][col] = rng.gen_range(0, 30);
        }
      }
    }
    map
  }
}
