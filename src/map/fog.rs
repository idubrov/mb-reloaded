use crate::map::{Cursor, MAP_COLS, MAP_ROWS};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct FogValue {
  value: u8,
}

impl FogValue {
  fn hidden() -> FogValue {
    FogValue { value: 1 }
  }

  pub fn reveal(&mut self) {
    self.value &= 0xfe;
  }
}

pub struct FogMap {
  data: Vec<FogValue>,
}

impl std::ops::Index<usize> for FogMap {
  type Output = [FogValue];

  fn index(&self, row: usize) -> &[FogValue] {
    &self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

impl std::ops::IndexMut<usize> for FogMap {
  fn index_mut(&mut self, row: usize) -> &mut [FogValue] {
    &mut self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

impl std::ops::Index<Cursor> for FogMap {
  type Output = FogValue;

  fn index(&self, cursor: Cursor) -> &FogValue {
    &self[cursor.row][cursor.col]
  }
}

impl std::ops::IndexMut<Cursor> for FogMap {
  fn index_mut(&mut self, cursor: Cursor) -> &mut FogValue {
    &mut self[cursor.row][cursor.col]
  }
}

impl FogMap {
  pub fn new() -> Self {
    Self {
      data: vec![FogValue::hidden(); MAP_COLS * MAP_ROWS],
    }
  }
}

impl Default for FogMap {
  fn default() -> Self {
    FogMap::new()
  }
}
