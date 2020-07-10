mod bitmaps;
mod level;

pub const MAP_ROWS: u16 = 45;
pub const MAP_COLS: u16 = 64;

use crate::world::position::Cursor;
pub use bitmaps::{CANNOT_PLACE_BOMB, CAN_EXTINGUISH, DIRT_BORDER_BITMAP, EXTINGUISHER_PASSABLE, PUSHABLE_BITMAP};
pub use level::{InvalidMap, LevelInfo, LevelMap, MapValue};
use rand::prelude::*;
use ref_cast::RefCast;

#[derive(Clone)]
pub struct Map<V> {
  data: Vec<V>,
}

#[derive(RefCast)]
#[repr(transparent)]
pub struct MapSlice<V> {
  slice: [V],
}

impl<V: Default + Copy> Default for Map<V> {
  fn default() -> Self {
    Map {
      data: vec![Default::default(); usize::from(MAP_COLS * MAP_ROWS)],
    }
  }
}

impl<V> std::ops::Index<u16> for Map<V> {
  type Output = MapSlice<V>;

  fn index(&self, row: u16) -> &MapSlice<V> {
    RefCast::ref_cast(&self.data[usize::from(row * MAP_COLS)..][..usize::from(MAP_COLS)])
  }
}

impl<V> std::ops::IndexMut<u16> for Map<V> {
  fn index_mut(&mut self, row: u16) -> &mut MapSlice<V> {
    RefCast::ref_cast_mut(&mut self.data[usize::from(row * MAP_COLS)..][..usize::from(MAP_COLS)])
  }
}

impl<V> std::ops::Index<u16> for MapSlice<V> {
  type Output = V;

  fn index(&self, col: u16) -> &V {
    &self.slice[usize::from(col)]
  }
}

impl<V> std::ops::IndexMut<u16> for MapSlice<V> {
  fn index_mut(&mut self, col: u16) -> &mut V {
    &mut self.slice[usize::from(col)]
  }
}

impl<V> std::ops::Index<Cursor> for Map<V> {
  type Output = V;

  fn index(&self, cursor: Cursor) -> &V {
    &self[cursor.row][cursor.col]
  }
}

impl<V> std::ops::IndexMut<Cursor> for Map<V> {
  fn index_mut(&mut self, cursor: Cursor) -> &mut V {
    &mut self[cursor.row][cursor.col]
  }
}

// Hits map

pub type HitsMap = Map<i32>;

impl Map<MapValue> {
  pub fn generate_hits_map(&self) -> HitsMap {
    let mut map = Map {
      data: vec![0i32; usize::from(MAP_COLS * MAP_ROWS)],
    };
    for cursor in Cursor::all() {
      map[cursor] = hits(self[cursor]);
    }
    map
  }
}

fn hits(value: MapValue) -> i32 {
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
    MapValue::Plastic | MapValue::Biomass => 400,
    MapValue::StoneLightCracked => 1000,
    MapValue::StoneHeavyCracked => 500,
    MapValue::Brick => 8000,
    MapValue::BrickLightCracked => 4000,
    MapValue::BrickHeavyCracked => 2000,
    _ => 0,
  }
}

// Fog map

pub type FogMap = Map<FogValue>;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct FogValue {
  value: u8,
}

impl FogValue {
  fn hidden() -> FogValue {
    FogValue { value: 1 }
  }

  pub fn reveal(&mut self) {
    self.value &= 0xfe;
  }
}

impl Default for FogValue {
  fn default() -> Self {
    FogValue::hidden()
  }
}

// Timer map

pub type TimerMap = Map<u16>;

impl Map<MapValue> {
  pub fn generate_timer_map(&self) -> TimerMap {
    let mut rng = rand::thread_rng();
    let mut map = Map {
      data: vec![0; usize::from(MAP_COLS * MAP_ROWS)],
    };
    for cursor in Cursor::all() {
      if self[cursor] == MapValue::Biomass {
        map[cursor] = rng.gen_range(0, 30);
      }
    }
    map
  }
}
