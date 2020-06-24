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
      other if CANNOT_PLACE_BOMB[self.maps.level[cursor]] => {
        // Cannot place bomb here!
        return;
      }
      _other => {
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
