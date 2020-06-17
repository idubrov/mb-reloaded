use crate::world::actor::{ActorComponent, ActorKind};
use crate::world::equipment::Equipment;
use crate::world::map::{FogMap, HitsMap, LevelMap, MapValue, TimerMap};
use crate::world::player::PlayerComponent;
use crate::world::position::{Cursor, Position};
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
