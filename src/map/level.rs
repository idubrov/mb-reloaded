use crate::entity::Direction;
use crate::map::{MAP_COLS, MAP_ROWS};
use num_enum::TryFromPrimitive;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Invalid map format")]
pub struct InvalidMap;

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

  pub fn cursor(&self, row: usize, col: usize) -> Cursor {
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
          && cursor[Direction::Right] == MapValue::Passage
          && cursor[Direction::Left] == MapValue::Passage
          && cursor[Direction::Up] == MapValue::Passage
          && cursor[Direction::Down] == MapValue::Passage
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
          if cursor[Direction::Right] == MapValue::Stone1
            && cursor[Direction::Down] == MapValue::Stone1
            && cursor[Direction::Left] == MapValue::Passage
            && cursor[Direction::Up] == MapValue::Passage
          {
            self[row][col] = MapValue::StoneTopLeft;
          } else if cursor[Direction::Right] == MapValue::Stone1
            && cursor[Direction::Down] == MapValue::Passage
            && cursor[Direction::Left] == MapValue::Passage
            && cursor[Direction::Up] == MapValue::Stone1
          {
            self[row][col] = MapValue::StoneBottomLeft;
          } else if cursor[Direction::Right] == MapValue::Passage
            && cursor[Direction::Down] == MapValue::Stone1
            && cursor[Direction::Left] == MapValue::Stone1
            && cursor[Direction::Up] == MapValue::Passage
          {
            self[row][col] = MapValue::StoneTopRight;
          } else if cursor[Direction::Right] == MapValue::Passage
            && cursor[Direction::Down] == MapValue::Passage
            && cursor[Direction::Left] == MapValue::Stone1
            && cursor[Direction::Up] == MapValue::Stone1
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
          if cursor[Direction::Right].is_stone_like()
            && cursor[Direction::Down].is_stone_like()
            && cursor[Direction::Left] == MapValue::Passage
            && cursor[Direction::Up] == MapValue::Passage
          {
            self[row][col] = MapValue::StoneTopLeft;
          } else if cursor[Direction::Right].is_stone_like()
            && cursor[Direction::Down] == MapValue::Passage
            && cursor[Direction::Left] == MapValue::Passage
            && cursor[Direction::Up].is_stone_like()
          {
            self[row][col] = MapValue::StoneBottomLeft;
          } else if cursor[Direction::Right] == MapValue::Passage
            && cursor[Direction::Down].is_stone_like()
            && cursor[Direction::Left].is_stone_like()
            && cursor[Direction::Up] == MapValue::Passage
          {
            self[row][col] = MapValue::StoneTopRight;
          } else if cursor[Direction::Right] == MapValue::Passage
            && cursor[Direction::Down] == MapValue::Passage
            && cursor[Direction::Left].is_stone_like()
            && cursor[Direction::Up].is_stone_like()
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
  Map00 = 0x00,
  Map01 = 0x01,
  Map02 = 0x02,
  Map03 = 0x03,
  Map04 = 0x04,
  Map05 = 0x05,
  Map06 = 0x06,
  Map07 = 0x07,
  Map08 = 0x08,
  Map09 = 0x09,
  Map0A = 0x0A,
  Map0B = 0x0B,
  Map0C = 0x0C,
  Map0D = 0x0D,
  Map0E = 0x0E,
  Map0F = 0x0F,
  Map10 = 0x10,
  Map11 = 0x11,
  Map12 = 0x12,
  Map13 = 0x13,
  Map14 = 0x14,
  Map15 = 0x15,
  Map16 = 0x16,
  Map17 = 0x17,
  Map18 = 0x18,
  Map19 = 0x19,
  Map1A = 0x1A,
  Map1B = 0x1B,
  Map1C = 0x1C,
  Map1D = 0x1D,
  Map1E = 0x1E,
  Map1F = 0x1F,
  Map20 = 0x20,
  Map21 = 0x21,
  Map22 = 0x22,
  Map23 = 0x23,
  Map24 = 0x24,
  Map25 = 0x25,
  Map26 = 0x26,
  Map27 = 0x27,
  Map28 = 0x28,
  Map29 = 0x29,
  Map2A = 0x2A,
  Map2B = 0x2B,
  Map2C = 0x2C,
  Map2D = 0x2D,
  Map2E = 0x2E,
  Map2F = 0x2F,
  Passage = 0x30,
  MetalWall = 0x31,
  Sand1 = 0x32,
  Sand2 = 0x33,
  Sand3 = 0x34,
  LightGravel = 0x35,
  HeavyGravel = 0x36,
  StoneTopLeft = 0x37,
  StoneTopRight = 0x38,
  StoneBottomRight = 0x39,
  Map3A = 0x3A,
  Map3B = 0x3B,
  Map3C = 0x3C,
  Map3D = 0x3D,
  Map3E = 0x3E,
  Map3F = 0x3F,
  Map40 = 0x40,
  StoneBottomLeft = 0x41,
  Boulder = 0x42,
  Stone1 = 0x43,
  Stone2 = 0x44,
  Stone3 = 0x45,
  Stone4 = 0x46,
  FurryRight = 0x47,
  FurryLeft = 0x48,
  FurryUp = 0x49,
  FurryDown = 0x4A,
  GrenadierRight = 0x4B,
  GrenadierLeft = 0x4C,
  GrenadierUp = 0x4D,
  GrenadierDown = 0x4E,
  SlimeRight = 0x4F,
  SlimeLeft = 0x50,
  SlimeUp = 0x51,
  SlimeDown = 0x52,
  AlienRight = 0x53,
  AlienLeft = 0x54,
  AlienUp = 0x55,
  AlienDown = 0x56,
  BombSmall1 = 0x57,
  BombBig1 = 0x58,
  BombDynamite1 = 0x59,
  Map5A = 0x5A,
  Map5B = 0x5B,
  Map5C = 0x5C,
  Map5D = 0x5D,
  Map5E = 0x5E,
  Map5F = 0x5F,
  Map60 = 0x60,
  Map61 = 0x61,
  Map62 = 0x62,
  SmallRadioBlue = 0x63,
  BigRadioBlue = 0x64,
  Mine = 0x65,
  Blood = 0x66,
  SmallRadioGreen = 0x67,
  BigRadioGreen = 0x68,
  SmallRadioYellow = 0x69,
  BigRadioYellow = 0x6A,
  Exit = 0x6B,
  Door = 0x6C,
  Medikit = 0x6D,
  Map6E = 0x6E,
  BioMass = 0x6F,
  StoneLightCracked = 0x70,
  StoneHeavyCracked = 0x71,
  Map72 = 0x72,
  Diamond,
  Map74 = 0x74,
  Map75 = 0x75,
  Map76 = 0x76,
  SmallBomb2 = 0x77,
  SmallBomb3 = 0x78,
  WeaponsCrate = 0x79,
  Map7A = 0x7A,
  Map7B = 0x7B,
  BarrelExtinguished = 0x7C,
  SmallBombExtinguished = 0x7D,
  BigBombExtinguished = 0x7E,
  Napalm1 = 0x7F,
  CrucifixBomb = 0x80,
  PlasticBomb = 0x81,
  SmallRadioRed = 0x82,
  BigRadioRed = 0x83,
  Map84 = 0x84,
  Map85 = 0x85,
  Map86 = 0x86,
  Map87 = 0x87,
  Map88 = 0x88,
  Map89 = 0x89,
  SmallCrucifixBomb = 0x8A,
  BigBomb2 = 0x8B,
  BigBomb3 = 0x8C,
  Dynamite2 = 0x8D,
  Dynamite3 = 0x8E,
  SmallPickaxe = 0x8F,
  LargePickaxe = 0x90,
  Drill = 0x91,
  GoldShield = 0x92,
  GoldEgg = 0x93,
  GoldPileCoins = 0x94,
  GoldBracelet = 0x95,
  GoldBar = 0x96,
  GoldCross = 0x97,
  GoldScepter = 0x98,
  GoldRubin = 0x99,
  GoldCrown = 0x9A,
  Plastic = 0x9B,
  Teleport = 0x9C,
  Atomic1 = 0x9D,
  Atomic2 = 0x9E,
  Atomic3 = 0x9F,
  ExplosivePlastic = 0xA0,
  BombExplosivePlastic = 0xA1,
  BombDigger = 0xA2,
  Napalm2 = 0xA3,
  Barrel = 0xA4,
  MapA5 = 0xA5,
  MapA6 = 0xA6,
  MapA7 = 0xA7,
  MapA8 = 0xA8,
  MapA9 = 0xA9,
  DynamiteExtinguished = 0xAA,
  JumpingBomb = 0xAB,
  Brick = 0xAC,
  BrickLightCracked = 0xAD,
  BrickHeavyCracked = 0xAE,
  SlimeCorpse = 0xAF,
  MapB0 = 0xB0,
  MapB1 = 0xB1,
  MapB2 = 0xB2,
  LifeItem = 0xB3,
  ButtonOff = 0xB4,
  ButtonOn = 0xB5,
  Item182 = 0xB6,
  MapB7 = 0xB7,
  MapB8 = 0xB8,
  MapB9 = 0xB9,
  MapBA = 0xBA,
  MapBB = 0xBB,
  MapBC = 0xBC,
  MapBD = 0xBD,
  MapBE = 0xBE,
  MapBF = 0xBF,
  MapC0 = 0xC0,
  MapC1 = 0xC1,
  MapC2 = 0xC2,
  MapC3 = 0xC3,
  MapC4 = 0xC4,
  MapC5 = 0xC5,
  MapC6 = 0xC6,
  MapC7 = 0xC7,
  MapC8 = 0xC8,
  MapC9 = 0xC9,
  MapCA = 0xCA,
  MapCB = 0xCB,
  MapCC = 0xCC,
  MapCD = 0xCD,
  MapCE = 0xCE,
  MapCF = 0xCF,
  MapD0 = 0xD0,
  MapD1 = 0xD1,
  MapD2 = 0xD2,
  MapD3 = 0xD3,
  MapD4 = 0xD4,
  MapD5 = 0xD5,
  MapD6 = 0xD6,
  MapD7 = 0xD7,
  MapD8 = 0xD8,
  MapD9 = 0xD9,
  MapDA = 0xDA,
  MapDB = 0xDB,
  MapDC = 0xDC,
  MapDD = 0xDD,
  MapDE = 0xDE,
  MapDF = 0xDF,
  MapE0 = 0xE0,
  MapE1 = 0xE1,
  MapE2 = 0xE2,
  MapE3 = 0xE3,
  MapE4 = 0xE4,
  MapE5 = 0xE5,
  MapE6 = 0xE6,
  MapE7 = 0xE7,
  MapE8 = 0xE8,
  MapE9 = 0xE9,
  MapEA = 0xEA,
  MapEB = 0xEB,
  MapEC = 0xEC,
  MapED = 0xED,
  MapEE = 0xEE,
  MapEF = 0xEF,
  MapF0 = 0xF0,
  MapF1 = 0xF1,
  MapF2 = 0xF2,
  MapF3 = 0xF3,
  MapF4 = 0xF4,
  MapF5 = 0xF5,
  MapF6 = 0xF6,
  MapF7 = 0xF7,
  MapF8 = 0xF8,
  MapF9 = 0xF9,
  MapFA = 0xFA,
  MapFB = 0xFB,
  MapFC = 0xFC,
  MapFD = 0xFD,
  MapFE = 0xFE,
  MapFF = 0xFF,
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

#[derive(Clone, Copy)]
pub struct Cursor<'m> {
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

impl std::ops::Index<Direction> for Cursor<'_> {
  type Output = MapValue;

  fn index(&self, dir: Direction) -> &Self::Output {
    let (row, col) = match dir {
      Direction::Left => (self.row, self.col - 1),
      Direction::Right => (self.row, self.col + 1),
      Direction::Up => (self.row - 1, self.col),
      Direction::Down => (self.row + 1, self.col),
    };
    &self.map[row][col]
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
