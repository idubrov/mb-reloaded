use crate::glyphs::Digging;
use crate::keys::Key;
use crate::world::actor::{ActorComponent, ActorKind};
use crate::world::equipment::Equipment;
use crate::world::map::{FogMap, HitsMap, LevelMap, MapValue, TimerMap, CANNOT_PLACE_BOMB};
use crate::world::player::PlayerComponent;
use crate::world::position::{Cursor, Direction, Position};
use rand::prelude::*;

pub mod actor;
pub mod equipment;
pub mod map;
pub mod player;
pub mod position;

pub struct Maps {
  pub darkness: bool,
  pub timer: TimerMap,
  pub level: LevelMap,
  pub hits: HitsMap,
  pub fog: FogMap,
}

pub struct World<'p> {
  pub maps: Maps,
  pub players: &'p mut [PlayerComponent],
  // First `players.len()` actors are players
  pub actors: Vec<ActorComponent>,
  pub shake: u32,

  ///
  pub update: UpdateQueue,
}

pub type EntityIndex = usize;

impl<'p> World<'p> {
  pub fn create(mut level: LevelMap, players: &'p mut [PlayerComponent], darkness: bool) -> Self {
    let mut actors = spawn_actors(&mut level, players.len());

    // Initialize players health and drilling power
    for (player_idx, player) in players.iter_mut().enumerate() {
      let actor = &mut actors[player_idx];
      actor.max_health = 100 + 100 * player.inventory[Equipment::Armor];
      actor.health = actor.max_health;
      actor.drilling = 1
        + player.inventory[Equipment::SmallPickaxe]
        + 3 * player.inventory[Equipment::LargePickaxe]
        + 5 * player.inventory[Equipment::Drill];

      // Reset player armor count
      player.inventory[Equipment::Armor] = 0;
    }

    World {
      maps: Maps {
        darkness,
        timer: level.generate_timer_map(),
        hits: level.generate_hits_map(),
        fog: FogMap::default(),
        level,
      },
      players,
      actors,
      shake: 0,
      update: Default::default(),
    }
  }

  /// Get player component if given entity is a player
  pub fn player_mut(&mut self, entity: EntityIndex) -> Option<&mut PlayerComponent> {
    self.players.get_mut(entity)
  }

  // If game is a single player game
  pub fn is_single_player(&self) -> bool {
    self.players.len() == 1
  }

  /// Count alive players
  pub fn alive_players(&self) -> usize {
    self.actors[0..self.players.len()]
      .iter()
      .filter(|actor| !actor.is_dead)
      .count()
  }

  pub fn update_super_drill(&mut self) {
    for actor in &mut self.actors[0..self.players.len()] {
      if actor.super_drill_count > 0 {
        actor.super_drill_count -= 1;
        if actor.super_drill_count == 0 {
          actor.drilling -= 300;
        }
      }
    }
  }

  pub fn player_action(&mut self, player: usize, key: Key) {
    let mut direction = None;
    let selection = self.players[player].selection;
    match key {
      Key::Up => {
        direction = Some(Direction::Up);
      }
      Key::Down => {
        direction = Some(Direction::Down);
      }
      Key::Left => {
        direction = Some(Direction::Left);
      }
      Key::Right => {
        direction = Some(Direction::Right);
      }
      Key::Stop => {
        self.actors[player].moving = false;
      }
      Key::Bomb => {
        self.activate_item(player);
      }
      Key::Choose => {
        let inventory = &self.players[player].inventory;
        let next = selection
          .selection_iter()
          .filter(|item| is_selectable(*item))
          .find(|item| inventory[*item] > 0)
          .unwrap_or(selection);
        self.players[player].selection = next;
        // FIXME: re-render selection and count!
      }
      Key::Remote => {
        unimplemented!();
      }
    }
    if let Some(direction) = direction {
      let mut actor = &mut self.actors[player];
      actor.facing = direction;
      actor.moving = true;
    }
  }

  fn activate_item(&mut self, player: usize) {
    let item = self.players[player].selection;

    if self.players[player].inventory[item] == 0 {
      // Nothing to use
      return;
    }

    let cursor = self.actors[player].pos.cursor();
    match item {
      Equipment::Flamethrower => {
        unimplemented!("flamethrower");
      }
      Equipment::Clone => {
        unimplemented!("activate clone");
      }
      Equipment::Extinguisher => {
        unimplemented!("extinguisher");
      }
      Equipment::SmallPickaxe | Equipment::LargePickaxe | Equipment::Drill | Equipment::Armor => {
        // Shouldn't really happen, but whatever.
        return;
      }
      Equipment::SuperDrill if self.actors[player].super_drill_count > 0 => {
        // Using already
        return;
      }
      Equipment::SuperDrill => {
        self.actors[player].super_drill_count = 10;
        self.actors[player].drilling += 300;
        return;
      }
      _other if CANNOT_PLACE_BOMB[self.maps.level[cursor]] => {
        // Cannot place bomb here!
        return;
      }
      item => {
        // Regular bombs case
        self.maps.level[cursor] = item_map_value(item, self.actors[player].facing, player);
        self.maps.timer[cursor] = item_initial_clock(item);

        // Some special handling for those bomb types
        match item {
          Equipment::JumpingBomb => {
            let mut rng = rand::thread_rng();
            self.maps.hits[cursor] = rng.gen_range(7, 27);
          }
          Equipment::Biomass => {
            self.maps.hits[cursor] = 400;
          }
          Equipment::Grenade => {
            self.maps.hits[cursor] = 20;
          }
          _ => {}
        }
      }
    }

    self.players[player].inventory[item] -= 1;
    self.players[player].stats.bombs_dropped += 1;
    // FIXME: render items count...
    // FIXME: reveal map square
  }

  pub fn detect_players(&mut self) {
    // FIXME: detect players
    // Visibility rules:

    let (players, monsters) = self.actors.split_at_mut(self.players.len());
    for monster in monsters {
      if monster.is_active {
        // Monster is ative already
        continue;
      }
      for player in players.iter() {
        if monster.is_active {
          // Monster is active already
          continue;
        }

        // 1. Closer than 20 in any direction (coordinate)
        // 2. On the same line & line of sight is not obstructed
        // 3. In forward field of view (90 degree fov) up to 7 cells distance
        monster.is_active = in_proximity(monster.pos, player.pos)
          || in_direct_sight(monster.pos.cursor(), player.pos.cursor(), &self.maps.level)
          || in_fov_sight(monster.pos.cursor(), player.pos.cursor(), monster.facing);
        if monster.is_active {
          // FIXME: play KARJAISU.VOC
          //  10300 frequency for alien
          //  11000 frequency for others
          eprintln!("monster activated!");
        }
      }
    }
  }
}

fn item_map_value(item: Equipment, direction: Direction, player: usize) -> MapValue {
  match item {
    Equipment::SmallBomb => MapValue::SmallBomb1,
    Equipment::BigBomb => MapValue::BigBomb1,
    Equipment::Dynamite => MapValue::Dynamite1,
    Equipment::AtomicBomb => MapValue::Atomic1,
    Equipment::SmallRadio => match player {
      0 => MapValue::SmallRadioBlue,
      1 => MapValue::SmallRadioRed,
      2 => MapValue::SmallRadioGreen,
      3 => MapValue::SmallRadioYellow,
      _ => unreachable!(),
    },
    Equipment::LargeRadio => match player {
      0 => MapValue::BigRadioBlue,
      1 => MapValue::BigRadioRed,
      2 => MapValue::BigRadioGreen,
      3 => MapValue::BigRadioYellow,
      _ => unreachable!(),
    },
    Equipment::Grenade => match direction {
      Direction::Left => MapValue::GrenadeFlyingLeft,
      Direction::Right => MapValue::GrenadeFlyingRight,
      Direction::Up => MapValue::GrenadeFlyingUp,
      Direction::Down => MapValue::GrenadeFlyingDown,
    },
    Equipment::Mine => MapValue::Mine,
    Equipment::Napalm => MapValue::Napalm1,
    Equipment::Barrel => MapValue::Barrel,
    Equipment::SmallCrucifix => MapValue::SmallCrucifixBomb,
    Equipment::LargeCrucifix => MapValue::LargeCrucifixBomb,
    Equipment::Plastic => MapValue::PlasticBomb,
    Equipment::ExplosivePlastic => MapValue::ExplosivePlasticBomb,
    Equipment::Digger => MapValue::DiggerBomb,
    Equipment::MetalWall => MapValue::MetalWall,
    Equipment::Teleport => MapValue::Teleport,
    Equipment::Biomass => MapValue::Biomass,
    Equipment::JumpingBomb => MapValue::JumpingBomb,
    Equipment::SmallPickaxe
    | Equipment::LargePickaxe
    | Equipment::Drill
    | Equipment::Flamethrower
    | Equipment::Extinguisher
    | Equipment::Armor
    | Equipment::SuperDrill
    | Equipment::Clone => {
      unreachable!();
    }
  }
}

fn item_initial_clock(item: Equipment) -> u16 {
  match item {
    Equipment::Mine | Equipment::SmallRadio | Equipment::LargeRadio | Equipment::Barrel | Equipment::Teleport => 0,
    Equipment::Napalm => 260,
    Equipment::AtomicBomb => 280,
    Equipment::ExplosivePlastic => 90,
    Equipment::Dynamite => 80,
    Equipment::JumpingBomb => {
      let mut rng = rand::thread_rng();
      rng.gen_range(80, 160)
    }
    Equipment::Biomass => {
      let mut rng = rand::thread_rng();
      rng.gen_range(0, 80)
    }
    Equipment::Grenade => 1,
    _ => 100,
  }
}

fn is_selectable(item: Equipment) -> bool {
  match item {
    Equipment::SmallPickaxe | Equipment::LargePickaxe | Equipment::Drill | Equipment::Armor => false,
    _ => true,
  }
}

fn spawn_actors(map: &mut LevelMap, players_count: usize) -> Vec<ActorComponent> {
  let mut actors = Vec::new();

  // Initialize players
  for player in 0..players_count {
    let kind = match player {
      0 => ActorKind::Player1,
      1 => ActorKind::Player2,
      2 => ActorKind::Player3,
      3 => ActorKind::Player4,
      _ => unimplemented!(),
    };
    actors.push(ActorComponent {
      kind,
      ..Default::default()
    });
  }
  init_players_positions(&mut actors);

  // Take all the monsters from the map and add them to the actors list
  for cursor in Cursor::all() {
    let value = map[cursor];
    if let Some((kind, facing)) = value.monster() {
      actors.push(ActorComponent {
        kind,
        pos: cursor.into(),
        health: kind.initial_health(),
        drilling: kind.drilling_power(),
        facing,
        ..Default::default()
      });

      // Remove monster from the map
      map[cursor] = MapValue::Passage;
    }
  }
  actors
}
fn init_players_positions(players: &mut [ActorComponent]) {
  let mut rng = rand::thread_rng();

  if players.len() == 1 {
    players[0].pos = Position::new(15, 45);
  } else {
    let mut rng = rand::thread_rng();

    if rng.gen::<bool>() {
      players[0].pos = Position::new(15, 45);
      players[1].pos = Position::new(625, 465);
    } else {
      players[0].pos = Position::new(625, 465);
      players[1].pos = Position::new(15, 45);
    }
  }

  if players.len() == 3 {
    if rng.gen::<bool>() {
      players[2].pos = Position::new(15, 465);
    } else {
      players[2].pos = Position::new(625, 45);
    }
  } else if players.len() == 4 {
    if rng.gen::<bool>() {
      players[2].pos = Position::new(15, 465);
      players[3].pos = Position::new(625, 45);
    } else {
      players[2].pos = Position::new(625, 45);
      players[3].pos = Position::new(15, 465);
    }
  }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Update {
  Actor(EntityIndex, Digging),
  Map(Cursor),
  Border(Cursor),
}

/// List of UI areas to update
#[derive(Default)]
pub struct UpdateQueue {
  /// Need to re-render players info
  pub players_info: bool,
  pub queue: Vec<Update>,
}

impl UpdateQueue {
  /// Need to re-render player lives
  pub fn update_player_lives(&mut self) {
    self.players_info = true;
  }

  /// Need to re-render player round stats (digging power and gold)
  pub fn update_player_stats(&mut self, _player: EntityIndex) {
    self.players_info = true;
  }

  /// Need to re-render player weapon selection and count
  pub fn update_player_selection(&mut self, _player: EntityIndex) {
    self.players_info = true;
  }

  /// Need to re-render player health
  pub fn update_player_health(&mut self, _player: EntityIndex) {
    self.players_info = true;
  }

  pub fn update_actor(&mut self, actor: EntityIndex, digging: Digging) {
    self.queue.push(Update::Actor(actor, digging));
  }

  pub fn update_cell(&mut self, cursor: Cursor) {
    self.queue.push(Update::Map(cursor));
  }

  pub fn update_cell_border(&mut self, cursor: Cursor) {
    self.queue.push(Update::Border(cursor));
  }
}

/// Check if two coordinates are in proximity to each other (less than 20 pixels in both direction)
fn in_proximity(first: Position, second: Position) -> bool {
  first.x < second.x + 20 && second.x < first.x + 20 && first.y < second.y + 20 && second.y < first.y + 20
}

/// Check if there is a direct line of sight between two cursor positions
fn in_direct_sight(first: Cursor, second: Cursor, level: &LevelMap) -> bool {
  if first.row == second.row {
    // Same row
    let mut cols = if first.col < second.col {
      first.col..=second.col
    } else {
      second.col..=first.col
    };
    cols.all(|col| level[first.row][col].is_passable())
  } else if first.col == second.col {
    let mut rows = if first.row < second.row {
      first.row..=second.row
    } else {
      second.row..=first.row
    };
    rows.all(|row| level[row][first.col].is_passable())
  } else {
    false
  }
}

/// Check if `first` looking in the `facing` direction will have `second` in its 7-cell field-of-view.
fn in_fov_sight(first: Cursor, second: Cursor, facing: Direction) -> bool {
  // (high, low): low and high coordinates in the direction of the view
  // (ortho1, ortho2): coordinates in the orthogonal dimension
  let (high, low, ortho1, ortho2) = match facing {
    Direction::Left => (first.col, second.col, second.row, first.row),
    Direction::Right => (second.col, first.col, second.row, first.row),
    Direction::Up => (first.row, second.row, second.col, first.col),
    Direction::Down => (second.row, first.row, second.col, first.col),
  };

  high >= low && high <= low + 7 && ortho2 + low < ortho1 + high && ortho1 + low < ortho2 + high
}
