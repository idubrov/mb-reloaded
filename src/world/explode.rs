use crate::bitmap;
use crate::bitmap::MapValueSet;
use crate::effects::SoundEffect;
use crate::world::map::{MapValue, MAP_ROWS};
use crate::world::position::{Cursor, Direction};
use crate::world::{SplatterKind, World};
use rand::prelude::*;

impl World<'_> {
  /// Activate entity in the cell (explode bomb, expand biomass, etc).
  pub(super) fn explode_entity(&mut self, cursor: Cursor, total: u32) {
    // Don't allow more than 200 bombs to explode at the same time
    if total > 200 {
      return;
    }

    let value = self.maps.level[cursor];
    match value {
      MapValue::MetalWall | MapValue::Door => {
        self.apply_damage_in_cell(cursor, 50);
      }
      MapValue::ButtonOff | MapValue::ButtonOn => {
        // Nothing
      }
      MapValue::MetalWallPlaced => {
        self.maps.level[cursor] = MapValue::MetalWall;
        self.update.update_cell(cursor);
        self.effects.play(SoundEffect::Picaxe, 11000, cursor);
        self.maps.hits[cursor] = 30_000;
      }
      MapValue::JumpingBomb => {
        self.explode_jumping_bomb(cursor, total);
      }
      MapValue::Barrel => {
        self.explode_barrel(cursor, total);
      }
      MapValue::GrenadeFlyingRight
      | MapValue::GrenadeFlyingLeft
      | MapValue::GrenadeFlyingDown
      | MapValue::GrenadeFlyingUp => self.grenade_fly(cursor, total),

      MapValue::Atomic1 | MapValue::Atomic2 | MapValue::Atomic3 => {
        self.maps.level[cursor] = MapValue::Passage;

        // Note: central square gets 2x damage!
        self.explode_cell(cursor, 255, true, total);

        // Note: as in original game, this is not exactly a circle due to improper rounding (ceil)
        for delta_col in -12..=12 {
          let cathet = f64::ceil(f64::sqrt(144.0 - (delta_col * delta_col) as f64)) as i16;
          for delta_row in -cathet..=cathet {
            if let Some(cursor) = cursor.offset(delta_row, delta_col) {
              self.explode_cell(cursor, 255, true, total);
            }
          }
        }

        self.effects.play(SoundEffect::Explos3, 5000, cursor);
        self.effects.play(SoundEffect::Explos3, 9900, cursor);
        self.effects.play(SoundEffect::Explos3, 10000, cursor);
        self.flash = true;
        self.shake = (self.shake + 10).min(MAP_ROWS);
      }

      MapValue::SmallBomb1
      | MapValue::SmallBomb2
      | MapValue::SmallBomb3
      | MapValue::Mine
      | MapValue::SmallBombExtinguished => {
        self.maps.level[cursor] = MapValue::Passage;
        self.explode_pattern(cursor, 60, &SMALL_BOMB_PATTERN, total);
        self.effects.play(SoundEffect::Pikkupom, 11000, cursor);
      }

      MapValue::SmallCrucifixBomb | MapValue::LargeCrucifixBomb => {
        let is_small = value == MapValue::SmallCrucifixBomb;
        self.maps.level[cursor] = MapValue::Passage;
        if is_small {
          self.explode_cell(cursor, 100, false, total);
        } else {
          self.explode_cell(cursor, 200, false, total);
        }

        for dir in Direction::all() {
          let mut cursor = cursor;
          for distance in 0.. {
            // Small bomb is limited to 15 squares
            if is_small && distance == 15 {
              break;
            }
            cursor = cursor.to(dir);

            match self.maps.level[cursor] {
              MapValue::MetalWall | MapValue::Exit | MapValue::Door | MapValue::ButtonOff | MapValue::ButtonOn => {
                break;
              }
              _ => {}
            }

            if is_small {
              self.explode_cell(cursor, 100, false, total);
            } else {
              self.explode_cell(cursor, 200, false, total);
            }
          }
        }

        let effect = if is_small {
          SoundEffect::Explos1
        } else {
          SoundEffect::Explos3
        };
        self.effects.play(effect, 11000, cursor);
      }
      MapValue::BigBomb1
      | MapValue::BigBomb2
      | MapValue::BigBomb3
      | MapValue::SmallRadioBlue
      | MapValue::SmallRadioRed
      | MapValue::SmallRadioGreen
      | MapValue::SmallRadioYellow
      | MapValue::ExplosivePlastic
      | MapValue::BigBombExtinguished => {
        self.maps.level[cursor] = MapValue::Passage;
        self.explode_pattern(cursor, 84, &BIG_BOMB_PATTERN, total);
        self.effects.play(SoundEffect::Explos1, 11000, cursor);
      }

      MapValue::Dynamite1
      | MapValue::Dynamite2
      | MapValue::Dynamite3
      | MapValue::BigRadioBlue
      | MapValue::BigRadioRed
      | MapValue::BigRadioGreen
      | MapValue::BigRadioYellow
      | MapValue::Teleport
      | MapValue::DynamiteExtinguished => {
        self.maps.level[cursor] = MapValue::Passage;
        self.explode_pattern(cursor, 100, &DYNAMITE_PATTERN, total);
        self.effects.play(SoundEffect::Explos2, 11000, cursor);
      }

      MapValue::ExplosivePlasticBomb => {
        self.expand_algo(&ExplodingPlasticExpansion, cursor, total);
        self.effects.play(SoundEffect::Urethan, 11000, cursor);
      }
      MapValue::DiggerBomb => {
        self.expand_algo(&DiggerExpansion, cursor, total);
        self.effects.play(SoundEffect::Explos5, 11000, cursor);
      }
      MapValue::Napalm1 | MapValue::Napalm2 | MapValue::NapalmExtinguished => {
        self.expand_algo(&NapalmExpansion, cursor, total);
        self.effects.play(SoundEffect::Explos5, 11000, cursor);
      }
      MapValue::PlasticBomb => {
        self.expand_algo(&PlasticExpansion, cursor, total);
        self.effects.play(SoundEffect::Urethan, 11000, cursor);
      }
      MapValue::Explosion => {
        self.maps.level[cursor] = MapValue::Smoke1;
        self.maps.timer[cursor] = 3;
        self.update.update_cell(cursor);
      }
      MapValue::Smoke1 => {
        self.maps.level[cursor] = MapValue::Smoke2;
        self.maps.timer[cursor] = 3;
        self.update.update_cell(cursor);
      }
      MapValue::Smoke2 => {
        self.maps.level[cursor] = MapValue::Passage;
        self.maps.timer[cursor] = 0;
        self.update.update_cell(cursor);
      }
      MapValue::MonsterDying => {
        self.maps.level[cursor] = MapValue::MonsterSmoke1;
        self.maps.timer[cursor] = 3;
        self.update.update_cell(cursor);
      }
      MapValue::MonsterSmoke1 => {
        self.maps.level[cursor] = MapValue::MonsterSmoke2;
        self.maps.timer[cursor] = 3;
        self.update.update_cell(cursor);
      }
      MapValue::MonsterSmoke2 => {
        self.maps.level[cursor] = MapValue::Blood;
        self.maps.timer[cursor] = 0;
        self.update.update_cell(cursor);

        for dir in Direction::all() {
          if can_splatter_blood(self.maps.level[cursor.to(dir)]) {
            self.update.update_splatter(cursor, dir, SplatterKind::Blood);
          }
        }
      }
      MapValue::SlimeDying => {
        self.maps.level[cursor] = MapValue::SlimeSmoke1;
        self.maps.timer[cursor] = 3;
        self.update.update_cell(cursor);
      }
      MapValue::SlimeSmoke1 => {
        self.maps.level[cursor] = MapValue::SlimeSmoke2;
        self.maps.timer[cursor] = 3;
        self.update.update_cell(cursor);
      }
      MapValue::SlimeSmoke2 => {
        self.maps.level[cursor] = MapValue::SlimeCorpse;
        self.maps.timer[cursor] = 0;
        self.update.update_cell(cursor);

        for dir in Direction::all() {
          if can_splatter_blood(self.maps.level[cursor.to(dir)]) {
            self.update.update_splatter(cursor, dir, SplatterKind::Slime);
          }
        }
      }
      MapValue::Biomass => {
        let mut rng = rand::thread_rng();
        let clock = rng.gen_range(1, 141);
        self.maps.timer[cursor] = clock;

        let dir = *[Direction::Left, Direction::Right, Direction::Up, Direction::Down]
          .choose(&mut rng)
          .unwrap();
        let cursor = cursor.to(dir);
        if self.maps.level[cursor].is_passable() {
          self.maps.level[cursor] = MapValue::Biomass;
          self.maps.timer[cursor] = clock;
          self.maps.hits[cursor] = 400;
          self.update.update_cell(cursor);
        }
      }

      _ => {
        // Nothing to do!
      }
    }
  }

  fn explode_jumping_bomb(&mut self, cursor: Cursor, total: u32) {
    let mut rng = rand::thread_rng();
    let bomb = *[MapValue::SmallBomb1, MapValue::BigBomb1, MapValue::Dynamite1]
      .choose(&mut rng)
      .unwrap();

    // Temporary place a bomb
    self.maps.level[cursor] = bomb;
    //self.update.update_cell(cursor);
    self.explode_entity(cursor, total + 1);

    let jumps = self.maps.hits[cursor];
    if jumps > 1 {
      let mut next = None;
      for _ in 0..6 {
        // Note that ranges are not symmetric as per original game!
        let delta_row = rng.gen_range(-4, 4);
        let delta_col = rng.gen_range(-4, 4);
        if let Some(cur) = cursor.offset(delta_row, delta_col) {
          let v = self.maps.level[cur];
          // FIXME: verify: cannot jump on blood; cannot jump on brick; cannot jump on cracked stone
          if v == MapValue::Passage
            || v.is_sand()
            || v.is_stone_corner()
            || v.is_stone()
            || v == MapValue::Boulder
            || v == MapValue::Explosion
          {
            next = Some(cur);
          }
        }
      }
      let next = next.unwrap_or(cursor);
      self.maps.level[next] = MapValue::JumpingBomb;
      self.maps.hits[cursor] = 0;
      self.maps.hits[next] = jumps - 1;
      self.update.update_cell(next);
      self.maps.timer[next] = rng.gen_range(1, 181);
    }
  }

  fn explode_barrel(&mut self, cursor: Cursor, total: u32) {
    let mut rng = rand::thread_rng();

    self.maps.level[cursor] = MapValue::Explosion;
    self.maps.timer[cursor] = 3;
    self.update.update_cell(cursor);

    self.effects.play(SoundEffect::Explos1, 11000, cursor);

    let from = rng.gen_range(0, 5);
    for _ in from..15 {
      let center = loop {
        // FIXME: again, non-symmetric
        let delta_col = rng.gen_range(-10, 10);
        let delta_row = rng.gen_range(-10, 10);
        if let Some(next) = cursor.offset(delta_row, delta_col) {
          break next;
        }
      };

      self.explode_pattern(center, 84, &BIG_BOMB_PATTERN, total);
      self.effects.play(SoundEffect::Explos1, 11000, center);
    }
  }

  fn grenade_fly(&mut self, cursor: Cursor, total: u32) {
    let value = self.maps.level[cursor];
    let dir = grenade_direction(value);
    let next = cursor.to(dir);

    // Either passable or another grenade flying in the same direction
    if (self.maps.level[next].is_passable() || value == self.maps.level[next]) && !self.apply_damage_in_cell(next, 0) {
      self.maps.level[cursor] = MapValue::Passage;
      self.reapply_blood(cursor);
      self.update.update_cell(cursor);

      self.maps.level[next] = value;
      self.update.update_cell(next);
      self.maps.timer[next] = 2;
    } else {
      self.maps.level[cursor] = MapValue::SmallBomb1;
      self.explode_entity(cursor, total);
    }
  }

  /// Explode cell via an external damage
  fn explode_cell(&mut self, cursor: Cursor, damage: u16, heavy_explosion: bool, total: u32) {
    let value = self.maps.level[cursor];
    if EXPLODABLE_ENTITY[value] {
      self.explode_entity(cursor, total);
    } else if value.is_stone() || value.is_stone_corner() || value == MapValue::Boulder {
      if heavy_explosion {
        self.maps.level[cursor] = MapValue::Explosion;
        self.maps.timer[cursor] = 3;
      } else {
        let mut rng = rand::thread_rng();
        if rng.gen::<bool>() {
          self.maps.level[cursor] = MapValue::StoneHeavyCracked;
          self.maps.hits[cursor] = 500;
        } else {
          self.maps.level[cursor] = MapValue::StoneLightCracked;
          self.maps.hits[cursor] = 1000;
        }
      }
    } else if value.is_brick_like() {
      if heavy_explosion {
        self.maps.level[cursor] = MapValue::Explosion;
        self.maps.timer[cursor] = 3;
      } else if value == MapValue::Brick {
        self.maps.hits[cursor] = 4000;
        self.maps.level[cursor] = MapValue::BrickLightCracked;
      } else if value == MapValue::BrickLightCracked {
        self.maps.hits[cursor] = 2000;
        self.maps.level[cursor] = MapValue::BrickHeavyCracked;
      } else if value == MapValue::BrickHeavyCracked {
        self.maps.level[cursor] = MapValue::Explosion;
        self.maps.timer[cursor] = 3;
      }
    } else {
      self.maps.level[cursor] = MapValue::Explosion;
      self.maps.timer[cursor] = 3;

      self.apply_damage_in_cell(cursor, damage);
    }

    self.update.update_cell(cursor);
    self.update.update_burned_border(cursor);
  }

  /// Generate an explosion given the pattern (list of row and collumn offsets). Note that pattern
  /// should not include the central square.
  fn explode_pattern(&mut self, center: Cursor, dmg: u16, pattern: &[(i16, i16)], total: u32) {
    self.explode_cell(center, dmg, false, total);
    for (delta_row, delta_col) in pattern {
      if let Some(cur) = center.offset(*delta_row, *delta_col) {
        self.explode_cell(cur, dmg, false, total);
      }
    }
  }

  /// Generic expansion algorithm used by plastic and digger
  fn expand_algo<E: Expansion>(&mut self, expansion: &E, cursor: Cursor, total: u32) {
    self.maps.level[cursor] = E::MARKER1;

    let mut expanded_count = 0;
    while expanded_count < E::MAX_EXPANSION {
      let mut spread = false;
      for cursor in Cursor::all_without_borders() {
        if self.maps.level[cursor] != E::MARKER1 {
          continue;
        }

        for dir in Direction::all() {
          let cursor = cursor.to(dir);
          let value = self.maps.level[cursor];
          if E::EXPLODE_ENTITIES && EXPLODABLE_ENTITY[value] {
            self.explode_entity(cursor, total);
          } else if expansion.can_expand(value, cursor, dir) {
            self.maps.level[cursor] = E::MARKER2;
            self.update.update_cell(cursor);
            expanded_count += 1;
            spread = true;
            expansion.expand(self, cursor);
          }
        }
      }

      // Haven't expanded even a single bit
      if !spread {
        break;
      }

      for cursor in Cursor::all() {
        if self.maps.level[cursor] == E::MARKER2 {
          self.maps.level[cursor] = E::MARKER1;
        }
      }
    }

    for cursor in Cursor::all() {
      if self.maps.level[cursor] == E::MARKER1 {
        expansion.finalize(self, cursor, total);
      }
    }
  }

  /// Fire a flamethrower
  pub(super) fn activate_flamethrower(&mut self, mut cursor: Cursor, direction: Direction) {
    self.effects.play(SoundEffect::Explos4, 11000, cursor);

    // If next cell is passable, start flame there (otherwise, start in current spot)
    // Note that if flame starts in current spot, it will destroy everything in that cell,
    // including metal walls (original behavior)!
    let value = self.maps.level[cursor.to(direction)];
    if is_flame_passable(value) {
      cursor = cursor.to(direction);
    }

    let expansion = FlamethrowerExpansion {
      start: cursor,
      direction,
    };
    self.expand_algo(&expansion, cursor, 0);
  }
}

fn grenade_direction(value: MapValue) -> Direction {
  match value {
    MapValue::GrenadeFlyingRight => Direction::Right,
    MapValue::GrenadeFlyingLeft => Direction::Left,
    MapValue::GrenadeFlyingDown => Direction::Down,
    MapValue::GrenadeFlyingUp => Direction::Up,
    _ => unreachable!(),
  }
}

/// Entity that can explode
pub const EXPLODABLE_ENTITY: MapValueSet = bitmap!([
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0010,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b1000_0000,
  0b0000_0011,
  0b1011_1000,
  0b0001_1111,
  0b1000_0000,
  0b1111_0001,
  0b0000_1111,
  0b0111_1100,
  0b0000_0000,
  0b1111_0000,
  0b1111_1111,
  0b0000_1111,
  0b0011_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
]);

/// Cross pattern of barrel explosion (these are offsets to row and column).
const BIG_BOMB_PATTERN: [(i16, i16); 12] = [
  (-1, 0),
  (1, 0),
  (0, -1),
  (0, 1),
  (-2, 0),
  (-1, 1),
  (0, 2),
  (1, 1),
  (2, 0),
  (1, -1),
  (0, -2),
  (-1, -1),
];

/// Cross pattern of small bomb explosion (these are offsets to row and column).
const SMALL_BOMB_PATTERN: [(i16, i16); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// Cross pattern of barrel explosion (these are offsets to row and column).
const DYNAMITE_PATTERN: [(i16, i16); 36] = [
  (-1, 0),
  (1, 0),
  (0, -1),
  (0, 1),
  (-2, 0),
  (-1, 1),
  (0, 2),
  (1, 1),
  (2, 0),
  (1, -1),
  (0, -2),
  (-1, -1),
  // 13
  (-3, 0),
  (-3, 1),
  (-2, 1),
  (-2, 2),
  (-1, 2),
  (-1, 3),
  (0, 3),
  (1, 3),
  (1, 2),
  (2, 2),
  (2, 1),
  (3, 1),
  (3, 0),
  (3, -1),
  (2, -1),
  (2, -2),
  (1, -2),
  (1, -3),
  (0, -3),
  (-1, -3),
  (-1, -2),
  (-2, -2),
  (-2, -1),
  (-3, -1),
];

/// Common trait for all expandable bombs (plastic, digger, napalm)
trait Expansion {
  const MARKER1: MapValue;
  const MARKER2: MapValue;
  const MAX_EXPANSION: u16;
  const EXPLODE_ENTITIES: bool;

  /// Check if can expand in the given square type. `next` is the position we expand to.
  /// `direction` is the direction of expansion.
  fn can_expand(&self, value: MapValue, next: Cursor, direction: Direction) -> bool;

  /// Additional work required to do when we expand into a cell
  fn expand(&self, _world: &mut World, _cursor: Cursor) {
    // By default, we do nothing extra
  }

  /// Update cell with the final result of expansion
  fn finalize(&self, world: &mut World, cursor: Cursor, total: u32);
}

struct ExplodingPlasticExpansion;

impl Expansion for ExplodingPlasticExpansion {
  const MARKER1: MapValue = MapValue::TempMarker1;
  const MARKER2: MapValue = MapValue::TempMarker2;
  const MAX_EXPANSION: u16 = 50;
  const EXPLODE_ENTITIES: bool = false;

  fn can_expand(&self, value: MapValue, _next: Cursor, _direction: Direction) -> bool {
    value.is_passable()
  }

  fn finalize(&self, world: &mut World, cursor: Cursor, _total: u32) {
    place_plastic(world, cursor, true);
  }
}

struct PlasticExpansion;

impl Expansion for PlasticExpansion {
  const MARKER1: MapValue = MapValue::TempMarker1;
  const MARKER2: MapValue = MapValue::TempMarker2;
  const MAX_EXPANSION: u16 = 45;
  const EXPLODE_ENTITIES: bool = false;

  fn can_expand(&self, value: MapValue, _next: Cursor, _direction: Direction) -> bool {
    value.is_passable()
  }

  fn finalize(&self, world: &mut World, cursor: Cursor, _total: u32) {
    place_plastic(world, cursor, false);
  }
}

struct DiggerExpansion;

impl Expansion for DiggerExpansion {
  const MARKER1: MapValue = MapValue::TempMarker1;
  const MARKER2: MapValue = MapValue::TempMarker2;
  const MAX_EXPANSION: u16 = 75;
  const EXPLODE_ENTITIES: bool = true;

  fn can_expand(&self, value: MapValue, _next: Cursor, _direction: Direction) -> bool {
    value.is_stone() || value.is_stone_corner() || value == MapValue::Boulder
  }

  fn finalize(&self, world: &mut World, cursor: Cursor, total: u32) {
    world.explode_cell(cursor, 10, true, total);
  }
}

struct NapalmExpansion;

impl Expansion for NapalmExpansion {
  const MARKER1: MapValue = MapValue::NapalmTempMarker1;
  const MARKER2: MapValue = MapValue::NapalmTempMarker2;
  const MAX_EXPANSION: u16 = 75;
  const EXPLODE_ENTITIES: bool = true;

  fn can_expand(&self, value: MapValue, _next: Cursor, _direction: Direction) -> bool {
    match value {
      MapValue::Passage
      | MapValue::Smoke1
      | MapValue::Smoke2
      | MapValue::Blood
      | MapValue::Biomass
      | MapValue::Explosion
      | MapValue::MonsterDying
      | MapValue::MonsterSmoke1
      | MapValue::MonsterSmoke2
      | MapValue::Plastic
      | MapValue::SlimeCorpse => true,
      _ => false,
    }
  }

  fn expand(&self, world: &mut World, cursor: Cursor) {
    // Burn everything in this cell
    world.maps.hits[cursor] = 0;
  }

  fn finalize(&self, world: &mut World, cursor: Cursor, total: u32) {
    world.maps.level[cursor] = MapValue::Passage;
    world.explode_cell(cursor, 220, true, total);
  }
}

struct FlamethrowerExpansion {
  /// Starting shooting direction
  start: Cursor,
  /// Flamethrower shooting direction
  direction: Direction,
}

impl Expansion for FlamethrowerExpansion {
  const MARKER1: MapValue = MapValue::NapalmTempMarker1;
  const MARKER2: MapValue = MapValue::NapalmTempMarker2;
  const MAX_EXPANSION: u16 = 30;
  const EXPLODE_ENTITIES: bool = true;

  fn can_expand(&self, value: MapValue, cursor: Cursor, direction: Direction) -> bool {
    if !is_flame_passable(value) {
      return false;
    }

    if self.direction == direction {
      return true;
    }
    if self.direction == direction.reverse() {
      return false;
    }

    let delta_col = if self.start.col > cursor.col {
      self.start.col - cursor.col
    } else {
      cursor.col - self.start.col
    };
    let delta_row = if self.start.row > cursor.row {
      self.start.row - cursor.row
    } else {
      cursor.row - self.start.row
    };

    // Expanding in direction perpendicular to player facing direction
    match direction {
      Direction::Up | Direction::Down => delta_row * 2 <= delta_col,
      Direction::Left | Direction::Right => delta_col * 2 <= delta_row,
    }
  }

  fn finalize(&self, world: &mut World, cursor: Cursor, total: u32) {
    world.maps.level[cursor] = MapValue::Passage;
    world.explode_cell(cursor, 34, true, total);
  }
}

/// Put given plastic value in the cell
fn place_plastic(world: &mut World, cursor: Cursor, explosive: bool) {
  // Original game is inconsistent here. Code only checks for player 1 and player 2.
  // However, the way check is written, it does not work for player 1 (it immediately
  // overrides square with `ExplosivePlastic`). Also, players 3 and 4 are not checked at all.
  // We fix that and make it work for every player
  if world.actors[..world.players.len()]
    .iter()
    .any(|actor| actor.pos.cursor() == cursor)
  {
    // Player is in this square: don't drop plastic here
    world.maps.level[cursor] = MapValue::Passage;
    world.maps.timer[cursor] = 0;
  } else if explosive {
    world.maps.level[cursor] = MapValue::ExplosivePlastic;
    world.maps.hits[cursor] = 400;
    world.maps.timer[cursor] = 250;
  } else {
    world.maps.level[cursor] = MapValue::Plastic;
    world.maps.hits[cursor] = 400;
    world.maps.timer[cursor] = 0;
  }
  world.update.update_cell(cursor);
}

fn can_splatter_blood(value: MapValue) -> bool {
  value == MapValue::MetalWall
    || value.is_sand()
    || value == MapValue::LightGravel
    || value == MapValue::HeavyGravel
    || value.is_stone_like()
    || value == MapValue::Biomass
    || value == MapValue::Plastic
    || value == MapValue::ExplosivePlastic
    || value.is_brick_like()
}

/// Check if square is something that flame can pass through
fn is_flame_passable(value: MapValue) -> bool {
  value.is_passable()
    || (value >= MapValue::Smoke1 && value <= MapValue::Smoke2)
    || value == MapValue::Biomass
    || (value >= MapValue::Explosion && value <= MapValue::MonsterSmoke2)
    || value == MapValue::Plastic
}
