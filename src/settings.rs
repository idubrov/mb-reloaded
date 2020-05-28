use crate::keys::KeysConfig;
use crate::map::LevelInfo;
use crate::options::Options;
use std::path::Path;
use std::rc::Rc;

pub struct GameSettings {
  pub keys: KeysConfig,
  pub levels: Vec<Rc<LevelInfo>>,
  pub options: Options,
}

impl GameSettings {
  /// Load game settings
  pub fn load(game_dir: &Path) -> Self {
    GameSettings {
      keys: KeysConfig::load(game_dir),
      levels: Vec::new(),
      options: Options::load(game_dir),
    }
  }
}
