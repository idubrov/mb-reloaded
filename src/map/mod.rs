pub mod bitmaps;
mod fog;
mod hits;
mod level;
mod timer;

pub const MAP_ROWS: usize = 45;
pub const MAP_COLS: usize = 64;

use crate::entity::Direction;
pub use fog::FogMap;
pub use hits::HitsMap;
pub use level::{InvalidMap, LevelInfo, LevelMap, MapValue};
pub use timer::TimerMap;

#[derive(Clone, Copy)]
pub struct Cursor {
  pub row: usize,
  pub col: usize,
}

impl Cursor {
  pub fn new(row: usize, col: usize) -> Cursor {
    Cursor { row, col }
  }

  pub fn to(self, dir: Direction) -> Cursor {
    let (row, col) = match dir {
      Direction::Left => (self.row, self.col - 1),
      Direction::Right => (self.row, self.col + 1),
      Direction::Up => (self.row - 1, self.col),
      Direction::Down => (self.row + 1, self.col),
    };
    Cursor { row, col }
  }
}
