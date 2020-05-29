use num_enum::TryFromPrimitive;
use rand::Rng;
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Invalid map format")]
pub struct InvalidMap;

const MAP_ROWS: usize = 45;
const MAP_COLS: usize = 64;

pub struct LevelMap {
  /// Map values
  data: Vec<MapValue>,
}

impl std::ops::Index<usize> for LevelMap {
  type Output = [MapValue];

  fn index(&self, row: usize) -> &[MapValue] {
    &self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

impl std::ops::IndexMut<usize> for LevelMap {
  fn index_mut(&mut self, row: usize) -> &mut [MapValue] {
    &mut self.data[row * MAP_COLS..][..MAP_COLS]
  }
}

pub enum LevelInfo {
  Random,
  File { name: String, map: LevelMap },
}

impl LevelMap {
  /// Create completely empty map
  pub fn empty() -> LevelMap {
    let mut data = Vec::new();
    data.resize(MAP_ROWS * MAP_COLS, MapValue::Passage);
    LevelMap { data }
  }

  /// Create statically typed map from a vector of bytes.
  pub fn from_file_map(external_map: Vec<u8>) -> Result<LevelMap, InvalidMap> {
    // Each map is 45 lines 66 bytes each (64 columns plus "\r\n" at the end of each row)
    if external_map.len() != 2970 {
      return Err(InvalidMap);
    }

    let mut data = Vec::with_capacity(MAP_ROWS * MAP_COLS);
    for row in 0..MAP_ROWS {
      // Two last bytes of the row are 0xd 0xa (newline), so 64 + 2 = 66
      let row = &external_map[row * (MAP_COLS + 2)..][..MAP_COLS];
      for value in row {
        // We could transmute here, but let's avoid all unsafe; amount of data is pretty small.
        data.push(MapValue::try_from(*value).unwrap());
      }
    }

    Ok(LevelMap { data })
  }

  /// Export map in the format used in map files
  pub fn to_file_map(&self) -> Vec<u8> {
    // Each map is 45 lines 66 bytes each (64 columns plus "\r\n" at the end of each row)
    let mut data = Vec::with_capacity(MAP_ROWS * (MAP_COLS + 2));
    for row in 0..MAP_ROWS {
      for col in 0..MAP_COLS {
        data.push(self[row][col] as u8);
      }
      data.push(b'\r');
      data.push(b'\n');
    }
    debug_assert_eq!(data.len(), 2970);
    data
  }

  /// Generate random stones on the map. This algorithm is close to the one used in the original
  /// game, but not exactly the same.
  pub fn random_stones(&mut self) {
    let mut rng = rand::thread_rng();
    for _ in 0..rng.gen_range(29, 40) {
      let mut col = rng.gen_range(1, MAP_COLS - 1);
      let mut row = rng.gen_range(1, MAP_ROWS - 1);
      loop {
        match rng.gen_range(0, 10) {
          0 => {
            self[row][col] = MapValue::Stone;
          }
          1 => {
            self[row][col] = MapValue::Stone;
            self[row + 1][col] = MapValue::Stone;
          }
          2 => {
            self[row][col] = MapValue::Stone;
            self[row - 1][col] = MapValue::Stone;
          }
          3 => {
            self[row][col] = MapValue::Stone;
            self[row][col + 1] = MapValue::Stone;
          }
          4 => {
            self[row][col] = MapValue::Stone;
            self[row - 1][col] = MapValue::Stone;
            self[row + 1][col] = MapValue::Stone;
          }
          5 => {
            self[row][col] = MapValue::Stone;
            self[row - 1][col] = MapValue::Stone;
            self[row + 1][col] = MapValue::Stone;
            self[row][col - 1] = MapValue::Stone;
          }
          6 => {
            self[row][col] = MapValue::Stone;
            self[row - 1][col] = MapValue::Stone;
            self[row + 1][col] = MapValue::Stone;
            self[row][col - 1] = MapValue::Stone;
            self[row][col + 1] = MapValue::Stone;
          }
          7 => {
            self[row][col] = MapValue::Stone;
            self[row - 1][col] = MapValue::Stone;
            self[row + 1][col] = MapValue::Stone;
            self[row][col - 1] = MapValue::Stone;
            self[row][col + 1] = MapValue::Stone;
          }
          8 => {
            self[row - 1][col] = MapValue::Stone;
            self[row + 1][col] = MapValue::Stone;
            self[row][col - 1] = MapValue::Stone;
            self[row - 1][col - 1] = MapValue::Stone;
            self[row + 1][col + 1] = MapValue::Stone;
            self[row + 1][col - 1] = MapValue::Stone;
            self[row - 1][col + 1] = MapValue::Stone;
          }
          // In original game, this seems to be never triggered as random number above is generated
          // in the range [0; 9) (end range is excluded). We, however, allow for this branch by
          // extending the random interval by one.
          9 => {
            self[row][col] = MapValue::Stone;
            self[row - 1][col] = MapValue::Stone;
            self[row + 1][col] = MapValue::Stone;
            self[row][col - 1] = MapValue::Stone;
            self[row][col + 1] = MapValue::Stone;
            self[row - 1][col - 1] = MapValue::Stone;
            self[row + 1][col + 1] = MapValue::Stone;
            self[row + 1][col - 1] = MapValue::Stone;
            self[row - 1][col + 1] = MapValue::Stone;
          }
          _ => {}
        }

        if rng.gen_range(0, 100) > rng.gen_range(93, 103) {
          break;
        }

        // Note: original game uses condition `col < 1` here. We use `col < 2` so we never get too close
        // to the border that one of the offsets above go outside of the map.
        if col < 2 {
          col += 1;
        } else if col >= MAP_COLS - 2 {
          col -= 1;
        } else {
          // Apply random offset -1, 0 or 1.
          col += 1;
          col -= rng.gen_range(0, 3);
        }

        // Note: original game uses condition `row < 1` here. We use `row < 2` so we never get too close
        // to the border that one of the offsets above go outside of the map.
        if row < 2 {
          row += 1;
        } else if row >= MAP_ROWS - 2 {
          row -= 1;
        } else {
          // Apply random offset -1, 0 or 1.
          row += 1;
          row -= rng.gen_range(0, 3);
        }
      }
    }
  }
}

/// Enum for all possible map values.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, TryFromPrimitive, PartialOrd)]
pub enum MapValue {
  Map00 = 0,
  Map01,
  Map02,
  Map03,
  Map04,
  Map05,
  Map06,
  Map07,
  Map08,
  Map09,
  Map0A,
  Map0B,
  Map0C,
  Map0D,
  Map0E,
  Map0F,
  Map10,
  Map11,
  Map12,
  Map13,
  Map14,
  Map15,
  Map16,
  Map17,
  Map18,
  Map19,
  Map1A,
  Map1B,
  Map1C,
  Map1D,
  Map1E,
  Map1F,
  Map20,
  Map21,
  Map22,
  Map23,
  Map24,
  Map25,
  Map26,
  Map27,
  Map28,
  Map29,
  Map2A,
  Map2B,
  Map2C,
  Map2D,
  Map2E,
  Map2F,
  /// 0x30
  Passage,
  /// 0x31
  MetalWall,
  Map32,
  Map33,
  Map34,
  Map35,
  Map36,
  Map37,
  Map38,
  Map39,
  Map3A,
  Map3B,
  Map3C,
  Map3D,
  Map3E,
  Map3F,
  Map40,
  Map41,
  Map42,
  /// 0x43
  Stone,
  Map44,
  Map45,
  Map46,
  Map47,
  Map48,
  Map49,
  Map4A,
  Map4B,
  Map4C,
  Map4D,
  Map4E,
  Map4F,
  Map50,
  Map51,
  Map52,
  Map53,
  Map54,
  Map55,
  Map56,
  Map57,
  Map58,
  Map59,
  Map5A,
  Map5B,
  Map5C,
  Map5D,
  Map5E,
  Map5F,
  Map60,
  Map61,
  Map62,
  Map63,
  Map64,
  Map65,
  Map66,
  Map67,
  Map68,
  Map69,
  Map6A,
  // Exit?
  Map6B,
  Map6C,
  Map6D,
  Map6E,
  BioMass,
  Map70,
  Map71,
  Map72,
  Map73,
  Map74,
  Map75,
  Map76,
  Map77,
  Map78,
  Map79,
  Map7A,
  Map7B,
  Map7C,
  Map7D,
  Map7E,
  Map7F,
  Map80,
  Map81,
  Map82,
  Map83,
  Map84,
  Map85,
  Map86,
  Map87,
  Map88,
  Map89,
  Map8A,
  Map8B,
  Map8C,
  Map8D,
  Map8E,
  Map8F,
  Map90,
  Map91,
  Map92,
  Map93,
  Map94,
  Map95,
  Map96,
  Map97,
  Map98,
  Map99,
  Map9A,
  Map9B,
  Map9C,
  Map9D,
  Map9E,
  Map9F,
  MapA0,
  MapA1,
  MapA2,
  MapA3,
  MapA4,
  MapA5,
  MapA6,
  MapA7,
  MapA8,
  MapA9,
  MapAA,
  MapAB,
  MapAC,
  MapAD,
  MapAE,
  MapAF,
  MapB0,
  MapB1,
  MapB2,
  MapB3,
  MapB4,
  MapB5,
  MapB6,
  MapB7,
  MapB8,
  MapB9,
  MapBA,
  MapBB,
  MapBC,
  MapBD,
  MapBE,
  MapBF,
  MapC0,
  MapC1,
  MapC2,
  MapC3,
  MapC4,
  MapC5,
  MapC6,
  MapC7,
  MapC8,
  MapC9,
  MapCA,
  MapCB,
  MapCC,
  MapCD,
  MapCE,
  MapCF,
  MapD0,
  MapD1,
  MapD2,
  MapD3,
  MapD4,
  MapD5,
  MapD6,
  MapD7,
  MapD8,
  MapD9,
  MapDA,
  MapDB,
  MapDC,
  MapDD,
  MapDE,
  MapDF,
  MapE0,
  MapE1,
  MapE2,
  MapE3,
  MapE4,
  MapE5,
  MapE6,
  MapE7,
  MapE8,
  MapE9,
  MapEA,
  MapEB,
  MapEC,
  MapED,
  MapEE,
  MapEF,
  MapF0,
  MapF1,
  MapF2,
  MapF3,
  MapF4,
  MapF5,
  MapF6,
  MapF7,
  MapF8,
  MapF9,
  MapFA,
  MapFB,
  MapFC,
  MapFD,
  MapFE,
  MapFF,
}
