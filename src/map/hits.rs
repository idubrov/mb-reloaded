use crate::map::{LevelMap, MapValue, MAP_COLS, MAP_ROWS};

pub struct HitsMap {
  data: Vec<u16>,
}

impl std::ops::Index<usize> for HitsMap {
  type Output = [u16];

  fn index(&self, row: usize) -> &[u16] {
    &self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

impl std::ops::IndexMut<usize> for HitsMap {
  fn index_mut(&mut self, row: usize) -> &mut [u16] {
    &mut self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

impl HitsMap {
  pub fn from_level_map(level_map: &LevelMap) -> Self {
    let mut map = Self {
      data: vec![0; MAP_COLS * MAP_ROWS],
    };
    for row in 0..MAP_ROWS {
      for col in 0..MAP_COLS {
        map[row][col] = hits(level_map[row][col]);
      }
    }
    map
  }
}

fn hits(value: MapValue) -> u16 {
  match value {
    MapValue::MetalWall => 30_000,
    MapValue::Sand1 => 22,
    MapValue::Sand2 => 23,
    MapValue::Sand3 => 24,
    MapValue::LightGravel => 108,
    MapValue::HeavyGravel => 347,
    MapValue::StoneTopLeft | MapValue::StoneTopRight | MapValue::StoneBottomRight | MapValue::StoneBottomLeft => 1227,
    MapValue::Boulder => 24,
    MapValue::Stone1 => 2000,
    MapValue::Stone2 => 2150,
    MapValue::Stone3 => 2200,
    MapValue::Stone4 => 2100,
    MapValue::Plastic | MapValue::BioMass => 400,
    MapValue::StoneLightCracked => 1000,
    MapValue::StoneHeavyCracked => 500,
    MapValue::Brick => 8000,
    MapValue::BrickLightCracked => 4000,
    MapValue::BrickHeavyCracked => 2000,

    _ => 0,
  }
}
