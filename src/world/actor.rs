use crate::effects::SoundEffect;
use crate::world::map::MapValue;
use crate::world::position::{Direction, Position};
use crate::world::EntityIndex;

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ActorKind {
  Furry,
  Grenadier,
  Slime,
  Alien,
  Player1,
  Player2,
  Player3,
  Player4,
}

impl ActorKind {
  pub fn drilling_power(self) -> u16 {
    match self {
      ActorKind::Furry => 5,
      ActorKind::Grenadier => 12,
      ActorKind::Slime => 12,
      ActorKind::Alien => 52,
      _ => unimplemented!(),
    }
  }

  pub fn initial_health(self) -> u16 {
    match self {
      ActorKind::Furry => 29,     // or 2
      ActorKind::Grenadier => 29, // or 3
      ActorKind::Slime => 10,     // or 1
      ActorKind::Alien => 66,     // or 5
      _ => unimplemented!(),
    }
  }

  pub fn speed(self) -> u16 {
    match self {
      ActorKind::Furry => 6,
      ActorKind::Grenadier => 3,
      ActorKind::Slime => 2,
      ActorKind::Alien => 100,
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