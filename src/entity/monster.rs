use crate::entity::{Direction, Inventory, PlayerEntity};
use crate::map::{Cursor, LevelMap, MapValue, MAP_COLS, MAP_ROWS};
use crate::roster::PlayerStats;

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MonsterKind {
  Furry,
  Grenadier,
  Slime,
  Alien,
  Player1,
  Player2,
  Player3,
  Player4,
}

impl MonsterKind {
  pub fn drilling_power(self) -> i32 {
    match self {
      MonsterKind::Furry => 5,
      MonsterKind::Grenadier => 12,
      MonsterKind::Slime => 12,
      MonsterKind::Alien => 52,
      _ => unimplemented!(),
    }
  }

  pub fn initial_health(self) -> u16 {
    match self {
      MonsterKind::Furry => 29,     // or 2
      MonsterKind::Grenadier => 29, // or 3
      MonsterKind::Slime => 10,     // or 1
      MonsterKind::Alien => 66,     // or 5
      _ => unimplemented!(),
    }
  }
}

const TEMPLATE: MonsterEntity = MonsterEntity {
  kind: MonsterKind::Furry,
  facing: Direction::Right,
  moving: None,
  health: 0,
  pos: Position { x: 0, y: 0 },
  drilling: 0,
  animation: 0,
  is_dead: false,
  inventory: Inventory::empty(),
  accumulated_cash: 0,
  lives: 0,
};

#[derive(Clone, Copy)]
pub struct Position {
  pub x: i32,
  pub y: i32,
}

impl Position {
  pub fn new(x: i32, y: i32) -> Self {
    Self { x, y }
  }
  /// Adjust coordinate to step in a given direction
  pub fn step(&mut self, dir: Direction) {
    match dir {
      Direction::Left => self.x -= 1,
      Direction::Right => self.x += 1,
      Direction::Up => self.y -= 1,
      Direction::Down => self.y += 1,
    }
  }

  /// Center the coordinate orthogonal to the moving direction
  pub fn center_orthogonal(&mut self, dir: Direction) {
    match dir {
      Direction::Left | Direction::Right => {
        self.y = (self.y / 10) * 10 + 5;
      }
      Direction::Up | Direction::Down => {
        self.x = (self.x / 10) * 10 + 5;
      }
    }
  }

  pub fn cursor(self) -> Cursor {
    let row = ((self.y - 30) / 10) as usize;
    let col = (self.x / 10) as usize;
    Cursor::new(row as usize, col as usize)
  }
}

#[derive(Clone)]
pub struct MonsterEntity {
  pub kind: MonsterKind,
  pub facing: Direction,
  // FIXME: make a boolean flag?
  pub moving: Option<Direction>,
  pub health: u16,
  pub pos: Position,
  pub drilling: i32,
  pub animation: u8,
  pub is_dead: bool,
  pub inventory: Inventory,
  /// Cash accumulated in the current level
  pub accumulated_cash: u32,
  pub lives: u8,
}

impl MonsterEntity {
  pub fn from_map(map: &mut LevelMap) -> Vec<MonsterEntity> {
    let mut list = Vec::new();
    for row in 0..MAP_ROWS {
      for col in 0..MAP_COLS {
        let value = map[row][col];
        let kind = match value {
          MapValue::FurryRight | MapValue::FurryLeft | MapValue::FurryUp | MapValue::FurryDown => MonsterKind::Furry,
          MapValue::GrenadierRight | MapValue::GrenadierLeft | MapValue::GrenadierUp | MapValue::GrenadierDown => {
            MonsterKind::Grenadier
          }
          MapValue::SlimeRight | MapValue::SlimeLeft | MapValue::SlimeUp | MapValue::SlimeDown => MonsterKind::Slime,
          MapValue::AlienRight | MapValue::AlienLeft | MapValue::AlienUp | MapValue::AlienDown => MonsterKind::Alien,
          _ => continue,
        };

        let mut entity = MonsterEntity {
          kind,
          pos: Position {
            x: (col * 10 + 5) as i32,
            y: (row * 10 + 35) as i32,
          },
          health: kind.initial_health(),
          drilling: kind.drilling_power(),
          ..TEMPLATE
        };

        entity.facing = match value {
          MapValue::FurryRight | MapValue::GrenadierRight | MapValue::SlimeRight | MapValue::AlienRight => {
            Direction::Right
          }
          MapValue::FurryLeft | MapValue::GrenadierLeft | MapValue::SlimeLeft | MapValue::AlienLeft => Direction::Left,
          MapValue::FurryUp | MapValue::GrenadierUp | MapValue::SlimeUp | MapValue::AlienUp => Direction::Up,
          MapValue::FurryDown | MapValue::GrenadierDown | MapValue::SlimeDown | MapValue::AlienDown => Direction::Down,
          _ => unreachable!(),
        };

        list.push(entity);

        // Remove monster from the map
        map[row][col] = MapValue::Passage;
      }
    }

    list
  }

  // FIXME: do we use the same for players or only for clones?
  pub fn player_stats(&mut self) -> Option<&mut PlayerStats> {
    None
  }

  /// Player which initiated this clone
  pub fn clone_player(&mut self) -> Option<&mut PlayerEntity> {
    None
  }
}
