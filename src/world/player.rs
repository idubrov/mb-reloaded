use crate::keys::KeyBindings;
use crate::options::Options;
use crate::roster::RosterInfo;
use crate::world::equipment::Equipment;

/// Component corresponding to the active player
#[derive(Default)]
pub struct PlayerComponent {
  /// Player name and statistics
  pub stats: RosterInfo,
  /// Index of the player in the players roster (PLAYERS.DAT).
  pub roster_index: u8,
  /// Player keybindings
  pub keys: KeyBindings,
  /// Cash that will not be lost on death. All accumulated cash is moved into this bucket if
  /// player survives level.
  pub cash: u32,
  /// Player inventory
  pub inventory: Inventory,
  /// Currently selected item
  pub selection: Equipment,
  /// For single player mode, tracks amount of lives player has. Not used in multi-player mode.
  pub lives: u32,
}

impl PlayerComponent {
  pub fn new(stats: RosterInfo, keys: KeyBindings, options: &Options) -> Self {
    PlayerComponent {
      stats,
      keys,
      cash: u32::from(options.cash),
      ..Default::default()
    }
  }

  pub fn initial_drilling_power(&self) -> u16 {
    1 + self.inventory[Equipment::SmallPickaxe]
      + 3 * self.inventory[Equipment::LargePickaxe]
      + 5 * self.inventory[Equipment::Drill]
  }
}

#[derive(Default, Clone, Copy)]
pub struct Inventory {
  inventory: [u16; Equipment::TOTAL],
}

impl std::ops::Index<Equipment> for Inventory {
  type Output = u16;

  fn index(&self, index: Equipment) -> &u16 {
    &self.inventory[index as usize]
  }
}

impl std::ops::IndexMut<Equipment> for Inventory {
  fn index_mut(&mut self, index: Equipment) -> &mut u16 {
    &mut self.inventory[index as usize]
  }
}
