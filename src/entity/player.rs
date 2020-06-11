use crate::entity::monster::Position;
use crate::keys::KeyBindings;
use crate::roster::PlayerStats;
use num_enum::TryFromPrimitive;
use std::convert::TryInto;

/// Types of equipment that could be stored in an inventory and bought in the shop. Note that
/// ordering is the same as shop ordering (left to right, top to bottom).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum Equipment {
  SmallBomb,
  LargeBomb,
  Dynamite,
  AtomicBomb,
  SmallRadio,
  LargeRadio,
  Grenade,
  Mine,
  Flamethrower,
  Napalm,
  Barrel,
  SmallCrucifix,
  LargeCrucifix,
  Plastic,
  ExplosivePlastic,
  Digger,
  MetalWall,
  SmallPickaxe,
  LargePickaxe,
  Drill,
  Teleport,
  Clone,
  Biomass,
  Extinguisher,
  Armor,
  JumpingBomb,
  SuperDrill,
}

impl Equipment {
  pub const TOTAL: usize = 27;

  const PRICES: [u32; Equipment::TOTAL] = [
    1, 3, 10, 650, 15, 65, 300, 25, 500, 80, 90, 35, 145, 15, 80, 120, 50, 400, 1100, 1600, 70, 400, 50, 80, 800, 95,
    575,
  ];

  pub fn all_equipment() -> impl Iterator<Item = Equipment> {
    (0..Self::TOTAL as u8).map(|v| v.try_into().unwrap())
  }

  pub fn base_price(self) -> u32 {
    Self::PRICES[self as usize]
  }
}

/// Selected player information
pub struct PlayerInfo {
  /// Index of the player in the players roster (PLAYERS.DAT).
  pub roster_index: u8,
  /// Player name
  pub name: String,
}

/// Active entity in the game (player); contains player inventory and all running stats..
pub struct PlayerEntity {
  pub player: PlayerInfo,
  pub keys: KeyBindings,
  pub base_drillingpower: u32,
  /// Cash accumulated in the current level
  pub accumulated_cash: u32,
  pub cash: u32,
  pub inventory: Inventory,
  pub stats: PlayerStats,
  pub selection: Equipment,
  pub is_dead: bool,
  /// (x, y), where each coordinate is pixel coordinate
  pub pos: Position,
}

impl PlayerEntity {
  pub fn drilling_power(&self) -> u32 {
    self.base_drillingpower
      + self.inventory[Equipment::SmallPickaxe]
      + 3 * self.inventory[Equipment::LargePickaxe]
      + 5 * self.inventory[Equipment::Drill]
  }

  pub fn cash(&self) -> u32 {
    self.accumulated_cash + self.cash
  }

  #[allow(dead_code)]
  pub fn total_health(&self) -> u32 {
    100 + 100 * self.inventory[Equipment::Armor]
  }
}

#[derive(Default, Clone, Copy)]
pub struct Inventory {
  inventory: [u32; Equipment::TOTAL],
}

impl Inventory {
  pub const fn empty() -> Self {
    Self {
      inventory: [0; Equipment::TOTAL],
    }
  }
}

impl std::ops::Index<Equipment> for Inventory {
  type Output = u32;

  fn index(&self, index: Equipment) -> &u32 {
    &self.inventory[index as usize]
  }
}

impl std::ops::IndexMut<Equipment> for Inventory {
  fn index_mut(&mut self, index: Equipment) -> &mut u32 {
    &mut self.inventory[index as usize]
  }
}

impl PlayerEntity {
  /// Create a new entity
  pub fn new(player: PlayerInfo, keys: KeyBindings, cash: u32) -> Self {
    Self {
      player,
      keys,
      base_drillingpower: 1,
      accumulated_cash: 0,
      cash,
      inventory: Default::default(),
      stats: Default::default(),
      selection: Equipment::SmallBomb,
      is_dead: false,
      pos: Position { x: 0, y: 0 },
    }
  }
}
