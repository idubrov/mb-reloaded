use crate::map::{MAP_COLS, MAP_ROWS};

pub struct FogMap {
  data: Vec<u8>,
}

impl std::ops::Index<usize> for FogMap {
  type Output = [u8];

  fn index(&self, row: usize) -> &[u8] {
    &self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

impl std::ops::IndexMut<usize> for FogMap {
  fn index_mut(&mut self, row: usize) -> &mut [u8] {
    &mut self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

impl FogMap {
  pub fn new() -> Self {
    Self {
      data: vec![1; MAP_COLS * MAP_ROWS],
    }
  }
}

impl Default for FogMap {
  fn default() -> Self {
    FogMap::new()
  }
}
