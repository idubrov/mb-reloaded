mod monster;
mod player;

pub use monster::MonsterEntity;
pub use player::{Equipment, Inventory, PlayerEntity, PlayerInfo};

#[derive(Clone, Copy)]
pub enum Direction {
  Left,
  Right,
  Up,
  Down,
}

impl Direction {
  pub fn all() -> impl Iterator<Item = Direction> {
    [Direction::Left, Direction::Right, Direction::Up, Direction::Down]
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
