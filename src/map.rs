use num_enum::TryFromPrimitive;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Invalid map format")]
pub struct InvalidMap;

const MAP_ROWS: usize = 45;
const MAP_COLS: usize = 64;

const RANDOM_TREASURES: [MapValue; 13] = [
  MapValue::SmallPickaxe,
  MapValue::LargePickaxe,
  MapValue::Drill,
  MapValue::GoldShield,
  MapValue::GoldEgg,
  MapValue::GoldPileCoins,
  MapValue::GoldBracelet,
  MapValue::GoldBar,
  MapValue::GoldCross,
  MapValue::GoldScepter,
  MapValue::GoldRubin,
  MapValue::GoldCrown,
  MapValue::Diamond,
];
const RANDOM_TREASURES_WEIGHTS: [usize; 13] = [18, 12, 8, 200, 200, 200, 200, 200, 180, 160, 140, 80, 3];

#[derive(Clone)]
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

  pub fn random_map(treasures: u8) -> Self {
    let mut map = LevelMap::empty();
    map.generate_random_stone();
    map.finalize_map();
    map.generate_treasures(treasures);
    map.generate_random_items();
    map.generate_borders();
    map
  }

  /// Generate random stones on the map. This algorithm is close to the one used in the original
  /// game, but not exactly the same.
  fn generate_random_stone(&mut self) {
    let mut rng = rand::thread_rng();
    for _ in 0..rng.gen_range(29, 40) {
      self.generate_stone_chunk();
    }
  }

  /// Generate one single stone chunk
  fn generate_stone_chunk(&mut self) {
    let mut rng = rand::thread_rng();
    let mut col = rng.gen_range(1, MAP_COLS - 1);
    let mut row = rng.gen_range(1, MAP_ROWS - 1);
    loop {
      match rng.gen_range(0, 10) {
        0 => {
          self[row][col] = MapValue::Stone1;
        }
        1 => {
          self[row][col] = MapValue::Stone1;
          self[row + 1][col] = MapValue::Stone1;
        }
        2 => {
          self[row][col] = MapValue::Stone1;
          self[row - 1][col] = MapValue::Stone1;
        }
        3 => {
          self[row][col] = MapValue::Stone1;
          self[row][col + 1] = MapValue::Stone1;
        }
        4 => {
          self[row][col] = MapValue::Stone1;
          self[row - 1][col] = MapValue::Stone1;
          self[row + 1][col] = MapValue::Stone1;
        }
        5 => {
          self[row][col] = MapValue::Stone1;
          self[row - 1][col] = MapValue::Stone1;
          self[row + 1][col] = MapValue::Stone1;
          self[row][col - 1] = MapValue::Stone1;
        }
        6 => {
          self[row][col] = MapValue::Stone1;
          self[row - 1][col] = MapValue::Stone1;
          self[row + 1][col] = MapValue::Stone1;
          self[row][col - 1] = MapValue::Stone1;
          self[row][col + 1] = MapValue::Stone1;
        }
        7 => {
          self[row][col] = MapValue::Stone1;
          self[row - 1][col] = MapValue::Stone1;
          self[row + 1][col] = MapValue::Stone1;
          self[row][col - 1] = MapValue::Stone1;
          self[row][col + 1] = MapValue::Stone1;
        }
        8 => {
          self[row - 1][col] = MapValue::Stone1;
          self[row + 1][col] = MapValue::Stone1;
          self[row][col - 1] = MapValue::Stone1;
          self[row - 1][col - 1] = MapValue::Stone1;
          self[row + 1][col + 1] = MapValue::Stone1;
          self[row + 1][col - 1] = MapValue::Stone1;
          self[row - 1][col + 1] = MapValue::Stone1;
        }
        // In original game, this seems to be never triggered as random number above is generated
        // in the range [0; 9) (end range is excluded). We, however, allow for this branch by
        // extending the random interval by one.
        9 => {
          self[row][col] = MapValue::Stone1;
          self[row - 1][col] = MapValue::Stone1;
          self[row + 1][col] = MapValue::Stone1;
          self[row][col - 1] = MapValue::Stone1;
          self[row][col + 1] = MapValue::Stone1;
          self[row - 1][col - 1] = MapValue::Stone1;
          self[row + 1][col + 1] = MapValue::Stone1;
          self[row + 1][col - 1] = MapValue::Stone1;
          self[row - 1][col + 1] = MapValue::Stone1;
        }
        _ => {}
      }

      // Randomized exit condition
      if rng.gen_range(0, 100) > rng.gen_range(93, 103) {
        break;
      }

      row = random_offset(row, MAP_ROWS);
      col = random_offset(col, MAP_COLS);
    }
  }

  fn cursor(&self, row: usize, col: usize) -> Cursor {
    Cursor { map: self, row, col }
  }

  /// Finalize stone corners, randomize stones and sand
  ///
  /// This function in particular was rewritten a bit compared to the original one (minor changes
  /// to make code more readable, result looks similar).
  fn finalize_map(&mut self) {
    let mut rng = rand::thread_rng();

    // Step 1: replace lonely stones with boulders
    for row in 1..MAP_ROWS - 1 {
      for col in 1..MAP_COLS - 1 {
        let cursor = self.cursor(row, col);
        if cursor.is_stone_like()
          && cursor.right() == MapValue::Passage
          && cursor.left() == MapValue::Passage
          && cursor.top() == MapValue::Passage
          && cursor.bottom() == MapValue::Passage
        {
          self[row][col] = MapValue::Boulder;
        }
      }
    }

    // Step 2: replace certain patterns of sand with rounded stone corners
    for row in 1..MAP_ROWS - 1 {
      for col in 1..MAP_COLS - 1 {
        if self[row][col] == MapValue::Passage {
          let cursor = self.cursor(row, col);
          if cursor.right() == MapValue::Stone1
            && cursor.bottom() == MapValue::Stone1
            && cursor.left() == MapValue::Passage
            && cursor.top() == MapValue::Passage
          {
            self[row][col] = MapValue::StoneTopLeft;
          } else if cursor.right() == MapValue::Stone1
            && cursor.bottom() == MapValue::Passage
            && cursor.left() == MapValue::Passage
            && cursor.top() == MapValue::Stone1
          {
            self[row][col] = MapValue::StoneBottomLeft;
          } else if cursor.right() == MapValue::Passage
            && cursor.bottom() == MapValue::Stone1
            && cursor.left() == MapValue::Stone1
            && cursor.top() == MapValue::Passage
          {
            self[row][col] = MapValue::StoneTopRight;
          } else if cursor.right() == MapValue::Passage
            && cursor.bottom() == MapValue::Passage
            && cursor.left() == MapValue::Stone1
            && cursor.top() == MapValue::Stone1
          {
            self[row][col] = MapValue::StoneBottomRight;
          }
        }
      }
    }

    // Step 3: round stone corners
    for row in 1..MAP_ROWS - 1 {
      for col in 1..MAP_COLS - 1 {
        let cursor = self.cursor(row, col);
        if self[row][col] == MapValue::Stone1 {
          if cursor.right().is_stone_like()
            && cursor.bottom().is_stone_like()
            && cursor.left() == MapValue::Passage
            && cursor.top() == MapValue::Passage
          {
            self[row][col] = MapValue::StoneTopLeft;
          } else if cursor.right().is_stone_like()
            && cursor.bottom() == MapValue::Passage
            && cursor.left() == MapValue::Passage
            && cursor.top().is_stone_like()
          {
            self[row][col] = MapValue::StoneBottomLeft;
          } else if cursor.right() == MapValue::Passage
            && cursor.bottom().is_stone_like()
            && cursor.left().is_stone_like()
            && cursor.top() == MapValue::Passage
          {
            self[row][col] = MapValue::StoneTopRight;
          } else if cursor.right() == MapValue::Passage
            && cursor.bottom() == MapValue::Passage
            && cursor.left().is_stone_like()
            && cursor.top().is_stone_like()
          {
            self[row][col] = MapValue::StoneBottomRight;
          }
        }
      }
    }

    // Step 4: randomize sand and stone
    for row in 0..MAP_ROWS {
      for col in 0..MAP_COLS {
        if self[row][col] == MapValue::Stone1 {
          self[row][col] = *[MapValue::Stone1, MapValue::Stone2, MapValue::Stone3, MapValue::Stone4]
            .choose(&mut rng)
            .unwrap();
        } else if self[row][col] == MapValue::Passage {
          self[row][col] = *[MapValue::Sand1, MapValue::Sand2, MapValue::Sand3]
            .choose(&mut rng)
            .unwrap();
        }
      }
    }

    // Step 5: place gravel
    for _ in 0..300 {
      let (row, col) = self.pick_random_coord(MapValue::is_sand);
      self[row][col] = *[MapValue::LightGravel, MapValue::HeavyGravel].choose(&mut rng).unwrap();
    }
  }

  /// Place treasures on the map
  fn generate_treasures(&mut self, treasures: u8) {
    let mut rng = rand::thread_rng();
    // Original game would randomize treasures, but "min treasures" is always the same as
    // "max treasures", so we don't bother calling random.

    let distribution = WeightedIndex::new(&RANDOM_TREASURES_WEIGHTS).unwrap();

    let mut treasures_in_stone = 0;
    for _ in 0..treasures {
      let item = RANDOM_TREASURES[distribution.sample(&mut rng)];

      // Once we placed 20 treasures into stone, we place remaining ones randomly
      if treasures_in_stone > 20 {
        let col = rng.gen_range(0, MAP_COLS);
        let row = rng.gen_range(0, MAP_ROWS);
        self[row][col] = item;
      } else {
        let (row, col) = self.pick_random_coord(MapValue::is_stone);
        self[row][col] = item;
        treasures_in_stone += 1;
      }
    }
  }

  /// Generate various random items
  fn generate_random_items(&mut self) {
    let mut rng = rand::thread_rng();
    while rng.gen_range(0, 100) > 70 {
      let col = rng.gen_range(1, MAP_COLS - 1);
      let row = rng.gen_range(1, MAP_ROWS - 1);
      self[row][col] = MapValue::Boulder;
    }

    while rng.gen_range(0, 100) > 70 {
      let col = rng.gen_range(1, MAP_COLS - 1);
      let row = rng.gen_range(1, MAP_ROWS - 1);
      self[row][col] = MapValue::WeaponsCrate;
    }

    while rng.gen_range(0, 100) > 65 {
      let col = rng.gen_range(1, MAP_COLS - 1);
      let row = rng.gen_range(1, MAP_ROWS - 1);
      self[row][col] = MapValue::Medikit;
    }

    while rng.gen_range(0, 100) > 70 {
      let col = rng.gen_range(1, MAP_COLS - 1);
      let row = rng.gen_range(1, MAP_ROWS - 1);
      self[row][col] = MapValue::Teleport;

      let col = rng.gen_range(1, MAP_COLS - 1);
      let row = rng.gen_range(1, MAP_ROWS - 1);
      self[row][col] = MapValue::Teleport;
    }
  }

  /// Generate metal borders around the map
  fn generate_borders(&mut self) {
    for row in 0..MAP_ROWS {
      self[row][0] = MapValue::MetalWall;
      self[row][MAP_COLS - 1] = MapValue::MetalWall;
    }

    for col in 0..MAP_COLS {
      self[0][col] = MapValue::MetalWall;
      self[MAP_ROWS - 1][col] = MapValue::MetalWall;
    }
  }

  /// Pick random coordinate such that its map value matches the predicate. Returns row and column.
  fn pick_random_coord(&self, predicate: impl Fn(MapValue) -> bool) -> (usize, usize) {
    let mut rng = rand::thread_rng();
    let mut col = rng.gen_range(0, MAP_COLS);
    let mut row = rng.gen_range(0, MAP_ROWS);

    for _ in 0..MAP_ROWS * MAP_COLS {
      if predicate(self[row][col]) {
        break;
      }

      if col < MAP_COLS - 1 {
        col += 1;
      } else {
        col = 0;
        row += 1;
      }
      if row > MAP_ROWS - 1 {
        col = rng.gen_range(0, MAP_COLS);
        row = rng.gen_range(0, MAP_ROWS);
      }
    }
    (row, col)
  }

  pub fn generate_entrances(&mut self, players: u8) {
    let mut rng = rand::thread_rng();

    // Top left
    let rnd = rng.gen_range(4, 10);
    for col in 1..=rnd {
      self[1][col] = MapValue::Passage;
    }
    let rnd = rng.gen_range(4, 10);
    for row in 1..=rnd {
      self[row][1] = MapValue::Passage;
    }

    // Bottom right
    let rnd = rng.gen_range(4, 10);
    for col in 1..=rnd {
      self[MAP_ROWS - 2][MAP_COLS - 1 - col] = MapValue::Passage;
    }
    let rnd = rng.gen_range(4, 10);
    for row in 1..=rnd {
      self[MAP_ROWS - 1 - row][MAP_COLS - 2] = MapValue::Passage;
    }

    if players > 2 {
      // Top right
      let rnd = rng.gen_range(4, 10);
      for col in 1..=rnd {
        self[1][MAP_COLS - 1 - col] = MapValue::Passage;
      }
      let rnd = rng.gen_range(4, 10);
      for row in 1..=rnd {
        self[row][MAP_COLS - 2] = MapValue::Passage;
      }

      // Bottom left
      let rnd = rng.gen_range(4, 10);
      for col in 1..=rnd {
        self[MAP_ROWS - 2][col] = MapValue::Passage;
      }
      let rnd = rng.gen_range(4, 10);
      for row in 1..=rnd {
        self[MAP_ROWS - 1 - row][1] = MapValue::Passage;
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
  /// 0x32
  Sand1,
  /// 0x33
  Sand2,
  /// 0x34
  Sand3,
  /// 0x35
  LightGravel,
  /// 0x36
  HeavyGravel,
  /// 0x37
  StoneTopLeft,
  /// 0x38
  StoneTopRight,
  /// 0x39
  StoneBottomRight,
  Map3A,
  Map3B,
  Map3C,
  Map3D,
  Map3E,
  Map3F,
  Map40,
  /// 0x41
  StoneBottomLeft,
  /// 0x42
  Boulder,
  /// 0x43
  Stone1,
  // 0x44
  Stone2,
  // 0x45
  Stone3,
  // 0x46
  Stone4,
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
  /// 0x6D
  Medikit,
  Map6E,
  BioMass,
  Map70,
  Map71,
  Map72,
  /// 0x73
  Diamond,
  Map74,
  Map75,
  Map76,
  Map77,
  Map78,
  /// 0x79
  WeaponsCrate,
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
  SmallPickaxe,
  LargePickaxe,
  Drill,
  GoldShield,
  GoldEgg,
  GoldPileCoins,
  GoldBracelet,
  GoldBar,
  GoldCross,
  GoldScepter,
  GoldRubin,
  GoldCrown,
  Map9B,
  /// 0x9C
  Teleport,
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

impl MapValue {
  /// Check if value is stone or any of the stone corners
  pub fn is_stone_like(self) -> bool {
    match self {
      MapValue::Stone1
      | MapValue::Stone2
      | MapValue::Stone3
      | MapValue::Stone4
      | MapValue::StoneTopLeft
      | MapValue::StoneTopRight
      | MapValue::StoneBottomLeft
      | MapValue::StoneBottomRight => true,
      _ => false,
    }
  }

  /// Check if value is stone
  pub fn is_stone(self) -> bool {
    match self {
      MapValue::Stone1 | MapValue::Stone2 | MapValue::Stone3 | MapValue::Stone4 => true,
      _ => false,
    }
  }

  /// Check if value is sand
  pub fn is_sand(self) -> bool {
    match self {
      MapValue::Sand1 | MapValue::Sand2 | MapValue::Sand3 => true,
      _ => false,
    }
  }
}

struct Cursor<'m> {
  map: &'m LevelMap,
  row: usize,
  col: usize,
}

impl std::ops::Deref for Cursor<'_> {
  type Target = MapValue;

  fn deref(&self) -> &MapValue {
    &self.map[self.row][self.col]
  }
}

impl Cursor<'_> {
  fn left(&self) -> MapValue {
    self.map[self.row][self.col - 1]
  }

  fn right(&self) -> MapValue {
    self.map[self.row][self.col + 1]
  }

  fn top(&self) -> MapValue {
    self.map[self.row - 1][self.col]
  }

  fn bottom(&self) -> MapValue {
    self.map[self.row + 1][self.col]
  }
}

/// Apply random offset to the coordinate
fn random_offset(mut coord: usize, max: usize) -> usize {
  let mut rng = rand::thread_rng();

  // Note: original game uses condition `x < 1` here (for both rows and columns). We use `x < 2` so
  // we never get too close to the border that one of the offsets above go outside of the map.
  if coord < 2 {
    coord += 1;
  } else if coord >= max - 2 {
    coord -= 1;
  } else {
    // Apply random offset -1, 0 or 1.
    coord += 1;
    coord -= rng.gen_range(0, 3);
  }
  coord
}
