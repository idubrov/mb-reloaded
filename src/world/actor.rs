use crate::effects::SoundEffect;
use crate::world::map::{LevelMap, MapValue};
use crate::world::position::{Cursor, Direction, Position};
use crate::world::EntityIndex;
use rand::prelude::*;
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Player {
  Player1,
  Player2,
  Player3,
  Player4,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ActorKind {
  Furry,
  Grenadier,
  Slime,
  Alien,
  Player(Player),
  Clone(Player),
}

impl ActorKind {
  pub fn drilling_power(self) -> u16 {
    match self {
      ActorKind::Furry => 5,
      ActorKind::Grenadier => 12,
      ActorKind::Slime => 12,
      ActorKind::Alien => 52,
      ActorKind::Clone(_) => 52,
      _ => unimplemented!(),
    }
  }

  pub fn initial_health(self) -> u16 {
    match self {
      ActorKind::Furry => 29,
      ActorKind::Grenadier => 29,
      ActorKind::Slime => 10,
      ActorKind::Alien => 66,
      ActorKind::Clone(_) => 100,
      _ => unimplemented!(),
    }
  }

  pub fn damage(self) -> u16 {
    match self {
      ActorKind::Furry => 2,
      ActorKind::Grenadier => 3,
      ActorKind::Slime => 1,
      ActorKind::Alien => 5,
      ActorKind::Clone(_) => 1,
      // Players don't do damage by hands!
      ActorKind::Player(_) => 0,
    }
  }

  pub fn speed(self) -> usize {
    match self {
      ActorKind::Furry => 6,
      ActorKind::Grenadier => 3,
      ActorKind::Slime => 2,
      ActorKind::Alien => 100,
      ActorKind::Clone(_) => 100,
      _ => unimplemented!(),
    }
  }

  pub fn blood_value(self) -> MapValue {
    match self {
      ActorKind::Slime => MapValue::SlimeCorpse,
      _ => MapValue::Blood,
    }
  }

  pub fn death_animation_value(self) -> MapValue {
    match self {
      ActorKind::Slime => MapValue::SlimeDying,
      _ => MapValue::MonsterDying,
    }
  }

  pub fn death_sound_effect(self) -> SoundEffect {
    match self {
      ActorKind::Slime => SoundEffect::Urethan,
      _ => SoundEffect::Aargh,
    }
  }
}

/// Actor component is an active entity on the map. It has position, visual representation,
/// digging power and health.
#[derive(Clone)]
pub struct ActorComponent {
  pub kind: ActorKind,
  pub facing: Direction,
  pub moving: bool,
  /// Maximum health
  pub max_health: u16,
  /// Current health
  pub health: u16,
  pub pos: Position,
  pub drilling: u16,
  pub animation: u8,
  pub is_dead: bool,
  /// If monster is active
  pub is_active: bool,
  pub owner: Option<EntityIndex>,
  /// Cash accumulated in the current map; will be lost on death.
  pub accumulated_cash: u32,
  /// Countdown of player activated acceleration bonus
  pub super_drill_count: u32,
}

impl Default for ActorComponent {
  fn default() -> Self {
    ActorComponent {
      kind: ActorKind::Furry,
      facing: Direction::Right,
      moving: false,
      max_health: 0,
      health: 0,
      pos: Position { x: 0, y: 0 },
      drilling: 0,
      animation: 0,
      is_dead: false,
      is_active: false,
      owner: None,
      accumulated_cash: 0,
      super_drill_count: 0,
    }
  }
}

impl ActorComponent {
  /// Check if we can continue moving in the current direction
  pub fn can_move(&self, level: &LevelMap) -> bool {
    let next = self.pos.cursor().to(self.facing);
    let value = level[next];
    value.is_passable() || value.is_sand() || value.is_treasure()
  }

  /// Actively avoid given location
  pub fn avoid_position(&mut self, bomb: Cursor, level: &LevelMap) {
    let cursor = self.pos.cursor();
    let mut rng = rand::thread_rng();
    let (delta_row, delta_col) = cursor.distance(bomb);

    if delta_col > delta_row || rng.gen_range(0, 100) < 3 {
      self.facing = match cursor.col.cmp(&bomb.col) {
        Ordering::Greater => Direction::Right,
        Ordering::Less => Direction::Left,
        Ordering::Equal => self.facing,
      };

      if !self.can_move(level) {
        self.facing = Direction::Down;
      }
      if !self.can_move(level) {
        self.facing = Direction::Up;
      }
    } else {
      self.facing = match cursor.row.cmp(&bomb.row) {
        Ordering::Greater => Direction::Down,
        Ordering::Less => Direction::Up,
        Ordering::Equal => self.facing,
      };

      if !self.can_move(level) {
        self.facing = Direction::Left;
      }
      if !self.can_move(level) {
        self.facing = Direction::Right;
      }
    }
    self.moving = true;
  }

  /// Actively avoid given location
  pub fn head_to_target(&mut self, target: Cursor, level: &LevelMap) {
    let cursor = self.pos.cursor();
    let (delta_row, delta_col) = cursor.distance(target);

    self.moving = true;

    // Try going for longer direction first
    if delta_col > delta_row {
      self.facing = match cursor.col.cmp(&target.col) {
        Ordering::Greater => Direction::Left,
        Ordering::Less => Direction::Right,
        Ordering::Equal => self.facing,
      };
    } else {
      self.facing = match cursor.row.cmp(&target.row) {
        Ordering::Greater => Direction::Up,
        Ordering::Less => Direction::Down,
        Ordering::Equal => self.facing,
      };
    }

    // If blocked, try going for shorter direction
    if !self.can_move(level) {
      if delta_col <= delta_row {
        self.facing = match cursor.col.cmp(&target.col) {
          Ordering::Greater => Direction::Left,
          Ordering::Less => Direction::Right,
          Ordering::Equal => self.facing,
        };
      } else {
        self.facing = match cursor.row.cmp(&target.row) {
          Ordering::Greater => Direction::Up,
          Ordering::Less => Direction::Down,
          Ordering::Equal => self.facing,
        };
      }
    }

    // If blocked, choose random direction!
    if !self.can_move(level) {
      let mut rng = rand::thread_rng();

      // Note that this is a bit different than other place we go in random direction
      // Here it is possible to choose "stop" randomly
      let dir = *[
        None,
        Some(Direction::Left),
        Some(Direction::Right),
        Some(Direction::Up),
        Some(Direction::Down),
      ]
      .choose(&mut rng)
      .unwrap();
      if let Some(dir) = dir {
        self.facing = dir;
      } else {
        self.moving = false;
      }
    }
  }
}
