mod monster;
mod player;

pub use monster::{MonsterEntity, MonsterKind};
pub use player::{Equipment, Inventory, PlayerEntity, PlayerInfo};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Direction {
  Right,
  Left,
  Up,
  Down,
}

impl Direction {
  pub fn all() -> impl Iterator<Item = Direction> {
    [Direction::Right, Direction::Left, Direction::Up, Direction::Down]
      .iter()
      .copied()
  }

  pub fn reverse(self) -> Self {
    match self {
      Direction::Left => Direction::Right,
      Direction::Right => Direction::Left,
      Direction::Up => Direction::Down,
      Direction::Down => Direction::Up,
    }
  }
}
