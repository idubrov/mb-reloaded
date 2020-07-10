use crate::effects::SoundEffect;
use crate::glyphs::Digging;
use crate::keys::Key;
use crate::world::actor::{ActorComponent, ActorKind};
use crate::world::equipment::Equipment;
use crate::world::map::{FogMap, HitsMap, LevelMap, MapValue, TimerMap, CANNOT_PLACE_BOMB, PUSHABLE_BITMAP};
use crate::world::player::PlayerComponent;
use crate::world::position::{Cursor, Direction, Position};
use rand::prelude::*;

pub mod actor;
pub mod equipment;
mod explode;
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
  /// If atomic flash should be displayed
  pub flash: bool,
  pub shake: u16,
  /// Frame counter. Incremented by 1 each tick. Not every process is invoked on every tick.
  pub round_counter: usize,
  /// Counter for the "end of round" condition
  pub end_round_counter: usize,
  /// View updates
  pub update: UpdateQueue,
  /// Sound effects to play
  pub effects: SoundEffectsQueue,
  /// Damage percentage (0..100)
  pub bomb_damage: u8,
}

/// Request to play sound effect at a given frequency and location
pub struct SoundRequest {
  pub effect: SoundEffect,
  pub frequency: i32,
  /// Position to play the effect in the world
  pub location: Cursor,
}

#[derive(Default)]
pub struct SoundEffectsQueue {
  pub queue: Vec<SoundRequest>,
}

impl SoundEffectsQueue {
  fn play(&mut self, effect: SoundEffect, frequency: i32, location: Cursor) {
    self.queue.push(SoundRequest {
      effect,
      frequency,
      location,
    });
  }
}

pub type EntityIndex = usize;

impl<'p> World<'p> {
  pub fn create(mut level: LevelMap, players: &'p mut [PlayerComponent], darkness: bool, bomb_damage: u8) -> Self {
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
      flash: false,
      shake: 0,
      round_counter: 0,
      end_round_counter: 0,
      update: Default::default(),
      effects: Default::default(),
      bomb_damage,
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

        self.update.update_player_selection(player);
      }
      Key::Remote => {
        for cursor in Cursor::all() {
          // Activate remote bombs for the player
          if is_remote_for(self.maps.level[cursor], player) {
            self.maps.timer[cursor] = 1;
          }
        }
      }
    }
    if let Some(direction) = direction {
      let mut actor = &mut self.actors[player];
      actor.facing = direction;
      actor.moving = true;
    }
  }

  /// Run on tick of update for the world state
  pub fn tick(&mut self) {
    self.flash = false;

    if self.round_counter % 18 == 0 {
      self.update_super_drill();
    }

    self.tick_bombs();
    if self.shake > 0 {
      self.shake -= 1;
    }

    if self.round_counter % 5 == 0 {
      if self.is_single_player() {
        if self.actors[0].is_dead {
          if self.end_round_counter == 0 {
            self.players[0].lives -= 1;
            // FIXME: end round
            if self.players[0].lives == 0 {
              // FIXME: end game
            }
            self.update.update_player_lives();
          } else {
            self.end_round_counter += 2;
          }
        }
      } else if self.alive_players() < 2 {
        self.end_round_counter += 3;
      }
    }

    // Animate players
    self.animate_players();

    if self.round_counter % 2 == 0 {
      self.check_dead_players();
    }

    if self.round_counter % 5 == 0 {
      self.monsters_detect_players();
    }

    // FIXME: animate_monsters

    if self.round_counter % 20 == 0 && !self.is_single_player() && self.gold_remaining() == 0 {
      self.end_round_counter += 20;
    }
    self.round_counter += 1;
  }

  /// Check end-of-round condition
  pub fn is_end_of_round(&self) -> bool {
    self.end_round_counter > 100
  }

  /// Check if still has gold remaining in the level
  fn gold_remaining(&self) -> u32 {
    let mut total = 0;
    for cursor in Cursor::all() {
      total += self.maps.level[cursor].gold_value();
    }
    total
  }

  /// Animate player actors
  fn animate_players(&mut self) {
    for monster in 0..self.players.len() {
      if !self.actors[monster].is_dead {
        self.animate_actor(monster);
        if self.actors[monster].super_drill_count > 0 {
          self.animate_actor(monster);
        }
      }
    }
  }

  fn check_dead_players(&mut self) {
    for player in 0..self.players.len() {
      let actor = &mut self.actors[player];
      if !actor.is_dead && actor.health < 1 {
        self.players[player].stats.deaths += 1;
        self.effects.play(SoundEffect::Aargh, 11000, actor.pos.cursor());
        let cursor = actor.pos.cursor();
        self.maps.level[cursor] = MapValue::Blood;
        self.update.update_cell(cursor);
      }
    }
  }

  fn update_super_drill(&mut self) {
    for actor in &mut self.actors[0..self.players.len()] {
      if actor.super_drill_count > 0 {
        actor.super_drill_count -= 1;
        if actor.super_drill_count == 0 {
          actor.drilling -= 300;
        }
      }
    }
  }

  /// Update bombs state
  fn tick_bombs(&mut self) {
    for cursor in Cursor::all() {
      match self.maps.timer[cursor] {
        0 => {
          // Not an active entity -- nothing to do!
        }
        1 => {
          self.maps.timer[cursor] = 0;
          // Some bombs might extinguish themselves
          if let Some(extinguished) = check_fuse_went_out(self.maps.level[cursor]) {
            self.maps.level[cursor] = extinguished;
            self.update.update_cell(cursor);
          } else {
            self.explode_entity(cursor, 0);
          }
        }
        clock => {
          // Countdown and update animation if needed
          self.maps.timer[cursor] = clock - 1;
          let replacement = match self.maps.level[cursor] {
            MapValue::SmallBomb1 if clock <= 60 => MapValue::SmallBomb2,
            MapValue::SmallBomb2 if clock <= 30 => MapValue::SmallBomb3,
            MapValue::BigBomb1 if clock <= 60 => MapValue::BigBomb2,
            MapValue::BigBomb2 if clock <= 30 => MapValue::BigBomb3,
            MapValue::Dynamite1 if clock <= 40 => MapValue::Dynamite2,
            MapValue::Dynamite2 if clock <= 20 => MapValue::Dynamite3,
            MapValue::Napalm1 => MapValue::Napalm2,
            MapValue::Napalm2 => MapValue::Napalm1,
            MapValue::Atomic1 => MapValue::Atomic2,
            MapValue::Atomic2 => MapValue::Atomic3,
            MapValue::Atomic3 => MapValue::Atomic1,
            _ => continue,
          };
          self.maps.level[cursor] = replacement;
          self.update.update_cell(cursor);
        }
      }
    }
  }

  /// Activate currently selected item for the given player
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
        self.maps.level[cursor] = item_placement_level(item, self.actors[player].facing, player);
        self.maps.timer[cursor] = item_placement_timer(item);
        self.maps.hits[cursor] = item_placement_hits(item);
      }
    }

    self.players[player].inventory[item] -= 1;
    self.players[player].stats.bombs_dropped += 1;
    // FIXME: render items count...
    // FIXME: reveal map square
  }

  // FIXME: make private
  pub fn monsters_detect_players(&mut self) {
    // FIXME: detect players
    // Visibility rules:

    let (players, monsters) = self.actors.split_at_mut(self.players.len());
    for monster in monsters {
      if monster.is_active {
        // Monster is active already
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
          let frequency = if monster.kind == ActorKind::Alien { 10300 } else { 11000 };
          self
            .effects
            .play(SoundEffect::Karjaisu, frequency, monster.pos.cursor());
        }
      }
    }
  }

  /// Interact with the map cell (dig it with a pickaxe, pick up gold, press buttons).
  #[allow(clippy::cognitive_complexity)]
  fn interact_map(&mut self, entity: EntityIndex, cursor: Cursor) {
    let value = self.maps.level[cursor];
    if value.is_passable() {
      if let Some(player) = self.players.get_mut(entity) {
        player.stats.meters_ran += 1;
        if self.maps.darkness {
          self.reveal_view();
        }
      }
    }

    if value == MapValue::Passage {
      // FIXME: temporary
    } else if value == MapValue::MetalWall
      || value.is_sand()
      || value.is_stone_like()
      || value.is_brick_like()
      || value == MapValue::Biomass
      || value == MapValue::Plastic
      || value == MapValue::ExplosivePlastic
      || value == MapValue::LightGravel
      || value == MapValue::HeavyGravel
    {
      let actor = &self.actors[entity];
      // Diggable squares
      // FIXME: use mapvalueset
      if self.maps.hits[cursor] == 30_000 {
        // 30_000 is a metal wall
      } else if self.maps.hits[cursor] > 1 {
        self.maps.hits[cursor] -= i32::from(actor.drilling);
        if value.is_stone_like() {
          if self.maps.hits[cursor] < 500 {
            if value.is_stone_corner() {
              self.maps.level[cursor] = MapValue::LightGravel;
            } else {
              self.maps.level[cursor] = MapValue::StoneHeavyCracked;
            }
            self.update.update_cell(cursor);
          } else if self.maps.hits[cursor] < 1000 {
            if value.is_stone_corner() {
              self.maps.level[cursor] = MapValue::HeavyGravel;
            } else {
              self.maps.level[cursor] = MapValue::StoneLightCracked;
            }
            self.update.update_cell(cursor);
          }
        } else if value.is_brick_like() {
          if self.maps.hits[cursor] <= 2000 {
            self.maps.level[cursor] = MapValue::BrickHeavyCracked;
          } else if self.maps.hits[cursor] <= 4000 {
            self.maps.level[cursor] = MapValue::BrickLightCracked;
          }
          self.update.update_cell(cursor);
          return;
        }
      } else {
        self.maps.hits[cursor] = 0;
        self.maps.level[cursor] = MapValue::Passage;
        self.update.update_cell(cursor);
        self.update.update_cell_border(cursor);
      }
    } else if value == MapValue::Diamond
      || (value >= MapValue::GoldShield && value <= MapValue::GoldCrown)
      || (value >= MapValue::SmallPickaxe && value <= MapValue::Drill)
    {
      let drill_value = match value {
        MapValue::SmallPickaxe => 1,
        MapValue::LargePickaxe => 3,
        MapValue::Drill => 5,
        _ => 0,
      };
      let gold_value = value.gold_value();

      let actor = &self.actors[entity];
      if let Some(player) = actor.owner {
        self.actors[player].drilling += drill_value;
        self.actors[player].accumulated_cash += gold_value;
      }

      self.actors[entity].drilling += drill_value;
      self.actors[entity].accumulated_cash = gold_value;

      if value >= MapValue::SmallPickaxe && value <= MapValue::Drill {
        self.effects.play(SoundEffect::Picaxe, 11000, cursor);
      } else {
        let mut rng = rand::thread_rng();
        let frequency = *[10000, 12599, 14983].choose(&mut rng).unwrap();
        self.effects.play(SoundEffect::Kili, frequency, cursor);
        if let Some(player) = self.player_mut(entity) {
          player.stats.treasures_collected += 1;
        }
      }

      self.maps.hits[cursor] = 0;
      self.maps.level[cursor] = MapValue::Passage;

      self.update.update_player_stats(entity);
      self.update.update_cell(cursor);
    } else if value == MapValue::Mine {
      // Activate the mine
      self.maps.timer[cursor] = 1;
    } else if PUSHABLE_BITMAP[value] {
      let actor = &self.actors[entity];
      // Go to the target position
      let target = cursor.to(actor.facing);
      if self.maps.hits[cursor] == 30_000 {
        // FIXME: wall shouldn't be pushable anyways?
      } else if self.maps.hits[cursor] > 1 {
        // Still need to push a little
        self.maps.hits[cursor] -= i32::from(actor.drilling);
      } else if self.maps.level[target].is_passable() {
        // Check if no actors are blocking the path
        if self.actors.iter().all(|p| p.is_dead || p.pos.cursor() != target) {
          // Push to `target` location
          self.maps.level[target] = self.maps.level[cursor];
          self.maps.timer[target] = self.maps.timer[cursor];
          self.maps.hits[target] = 24;

          // Clear old position
          self.maps.level[cursor] = MapValue::Passage;
          self.maps.timer[cursor] = 0;

          // FIXME: re-render blood
          self.reapply_blood(cursor);

          self.update.update_cell(cursor);
          self.update.update_cell(target);
        }
      }
    } else if value == MapValue::WeaponsCrate {
      let mut rng = rand::thread_rng();
      match rng.gen_range(0, 5) {
        0 => {
          let cnt = rng.gen_range(1, 3);
          let weapon = *[
            Equipment::AtomicBomb,
            Equipment::Grenade,
            Equipment::Flamethrower,
            Equipment::Clone,
          ]
          .choose(&mut rng)
          .unwrap();
          if let Some(player) = self.player_mut(entity) {
            player.inventory[weapon] += cnt;
          }
        }
        1 => {
          let cnt = rng.gen_range(1, 6);
          let weapon = *[
            Equipment::Napalm,
            Equipment::LargeCrucifix,
            Equipment::Teleport,
            Equipment::Biomass,
            Equipment::Extinguisher,
            Equipment::JumpingBomb,
            Equipment::SuperDrill,
          ]
          .choose(&mut rng)
          .unwrap();
          if let Some(player) = self.player_mut(entity) {
            player.inventory[weapon] += cnt;
          }
        }
        _ => {
          let cnt = rng.gen_range(3, 13);
          let weapon = *[
            Equipment::SmallBomb,
            Equipment::BigBomb,
            Equipment::Dynamite,
            Equipment::SmallRadio,
            Equipment::LargeRadio,
            Equipment::Mine,
            Equipment::Barrel,
            Equipment::SmallCrucifix,
            Equipment::Plastic,
            Equipment::ExplosivePlastic,
            Equipment::Digger,
            Equipment::MetalWall,
          ]
          .choose(&mut rng)
          .unwrap();
          if let Some(player) = self.player_mut(entity) {
            player.inventory[weapon] += cnt;
          }
        }
      }

      self.maps.hits[cursor] = 0;
      self.maps.level[cursor] = MapValue::Passage;

      self.update.update_player_selection(entity);
      self.update.update_cell(cursor);
      self.effects.play(SoundEffect::Picaxe, 11000, cursor);
    } else if value == MapValue::LifeItem {
      if self.actors[entity].kind == ActorKind::Player1 {
        self.players[0].lives += 1;
        self.update.update_player_lives();
      }

      self.maps.hits[cursor] = 0;
      self.maps.level[cursor] = MapValue::Passage;

      self.update.update_cell(cursor);
    } else if value == MapValue::ButtonOff {
      if self.maps.timer[cursor] <= 1 {
        self.open_doors();
      }
    } else if value == MapValue::ButtonOn {
      if self.maps.timer[cursor] <= 1 {
        self.close_doors();
      }
    } else if value == MapValue::Teleport {
      let mut entrance_idx = 0;
      let mut teleport_count = 0;
      for cur in Cursor::all() {
        if self.maps.level[cur] == MapValue::Teleport {
          if cursor == cur {
            entrance_idx = teleport_count;
          }
          teleport_count += 1;
        }
      }

      let mut rng = rand::thread_rng();
      // FIXME: if teleport_count == 1
      let mut exit = rng.gen_range(0, teleport_count - 1);
      if exit >= entrance_idx {
        exit += 1;
      }

      for cur in Cursor::all() {
        if self.maps.level[cur] == MapValue::Teleport {
          if exit == 0 {
            // Found exit point
            let actor = &mut self.actors[entity];
            self.update.update_cell(actor.pos.cursor());

            // Move to the exit point
            actor.pos = cur.into();
            self.update.update_cell(actor.pos.cursor());
            break;
          }
          exit -= 1;
        }
      }
    } else if value == MapValue::Exit {
      unimplemented!("level exit");
    } else if value == MapValue::Medikit {
      if self.actors[entity].is_active {
        self.actors[entity].health = self.actors[entity].max_health;
      }

      self.maps.level[cursor] = MapValue::Passage;
      self.update.update_player_health(entity);
      self.update.update_cell(cursor);
      self.effects.play(SoundEffect::Picaxe, 11000, cursor);
    }
  }

  /// Re-apply blood / slime corpse to the map cell. Iterates through all of the actors and places
  /// blood / slime corpse at the cell if dead actors are found.
  fn reapply_blood(&mut self, cursor: Cursor) {
    self.apply_damage_in_cell(cursor, 0);
  }

  /// Apply damage to all actors in the cell. Returns `true` if found live actor in that cell.
  fn apply_damage_in_cell(&mut self, cursor: Cursor, dmg: u16) -> bool {
    let mut found_alive = false;
    for idx in 0..self.actors.len() {
      let actor = &self.actors[idx];
      if actor.pos.cursor() != cursor {
        continue;
      }

      let effective_dmg = match actor.kind {
        // In single player, damage is always 100%
        ActorKind::Player1 if self.is_single_player() => dmg,
        ActorKind::Player1 | ActorKind::Player2 | ActorKind::Player3 | ActorKind::Player4 => {
          dmg * u16::from(self.bomb_damage) / 100
        }
        _ => dmg,
      };
      // Get mutable
      let actor = &mut self.actors[idx];
      actor.health = actor.health.saturating_sub(effective_dmg);

      if idx < self.players.len() {
        self.update.update_player_health(idx);
      }

      found_alive |= !actor.is_dead;
      if actor.health == 0 {
        if dmg > 0 {
          self.maps.level[cursor] = actor.kind.death_animation_value();
          self.maps.timer[cursor] = 3;
        } else {
          self.maps.level[cursor] = actor.kind.blood_value();
        }
        if !actor.is_dead {
          if idx < self.players.len() {
            self.players[idx].stats.deaths += 1;
          }
          actor.is_dead = true;
          self.effects.play(actor.kind.death_sound_effect(), 11000, cursor);
        }
      }
    }
    found_alive
  }

  fn open_doors(&mut self) {
    unimplemented!()
  }

  fn close_doors(&mut self) {
    unimplemented!()
  }

  /// Animate actor under a given index. Updates coordinates, animation phase.
  fn animate_actor(&mut self, entity: EntityIndex) {
    let actor = &mut self.actors[entity];
    if !actor.moving {
      self.update.update_actor(entity, Digging::Hands);
      return;
    };

    let delta_x = actor.pos.x % 10;
    let delta_y = actor.pos.y % 10;
    let cursor = actor.pos.cursor();
    let direction = actor.facing;

    let (delta_dir, delta_orthogonal, finishing_move, can_move) = match direction {
      Direction::Left => (delta_x, delta_y, delta_x > 5, actor.pos.x > 5),
      Direction::Right => (delta_x, delta_y, delta_x < 5, actor.pos.x < 635),
      Direction::Up => (delta_y, delta_x, delta_y > 5, actor.pos.y > 35),
      Direction::Down => (delta_y, delta_x, delta_y < 5, actor.pos.y < 475),
    };

    // Vertically centered enough to be moving in the current direction
    let is_moving = can_move && delta_orthogonal > 3 && delta_orthogonal < 6;
    let map_value = self.maps.level[cursor.to(direction)];
    // Either finishing move into the cell or cell to the left is passable
    if is_moving && (finishing_move || map_value.is_passable()) {
      actor.pos.step(direction);
    }

    if delta_orthogonal != 5 {
      // Center our position in orthogonal direction
      actor.pos.center_orthogonal(direction);

      // Need to redraw cell orthogonal to the moving direction if we are re-centering.
      let cur = match direction {
        Direction::Left | Direction::Right if delta_orthogonal > 5 => cursor.to(Direction::Down),
        Direction::Left | Direction::Right => cursor.to(Direction::Up),
        Direction::Up | Direction::Down if delta_orthogonal > 5 => cursor.to(Direction::Right),
        Direction::Up | Direction::Down => cursor.to(Direction::Left),
      };
      self.update.update_cell(cur);
    }

    // We are centered in the direction we are going -- hit the map!
    if delta_dir == 5 {
      self.interact_map(entity, cursor.to(direction));
    }

    // Finishing moving from adjacent square -- render that square
    if finishing_move {
      self.update.update_cell(cursor.to(direction.reverse()));
    }

    // Check if we need to show animation with pick axe or without
    let is_hard = delta_dir == 5
      && ((map_value >= MapValue::StoneTopLeft && map_value <= MapValue::StoneBottomRight)
        || map_value == MapValue::StoneBottomLeft
        || (map_value >= MapValue::Stone1 && map_value <= MapValue::Stone4)
        || (map_value >= MapValue::StoneLightCracked && map_value <= MapValue::StoneHeavyCracked)
        || (map_value >= MapValue::Brick && map_value <= MapValue::BrickHeavyCracked));
    let digging = if is_hard { Digging::Pickaxe } else { Digging::Hands };

    self.update.update_actor(entity, digging);

    let actor = &mut self.actors[entity];
    actor.animation %= 30;
    if digging == Digging::Pickaxe && actor.animation == 16 {
      let mut rng = rand::thread_rng();
      let frequency = rng.gen_range(11000, 11100);
      self.effects.play(SoundEffect::Picaxe, frequency, cursor);
    }
    actor.animation += 1;
  }

  /// Reveal map based on player vision
  fn reveal_view(&mut self) {
    unimplemented!("reveal view")
  }
}

fn item_placement_level(item: Equipment, direction: Direction, player: usize) -> MapValue {
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
    Equipment::MetalWall => MapValue::MetalWallPlaced,
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

fn is_remote_for(value: MapValue, player: EntityIndex) -> bool {
  match value {
    MapValue::SmallRadioBlue | MapValue::BigRadioBlue if player == 0 => true,
    MapValue::SmallRadioRed | MapValue::BigRadioRed if player == 1 => true,
    MapValue::SmallRadioGreen | MapValue::BigRadioGreen if player == 2 => true,
    MapValue::SmallRadioYellow | MapValue::BigRadioYellow if player == 3 => true,
    _ => false,
  }
}

fn item_placement_timer(item: Equipment) -> u16 {
  match item {
    Equipment::Mine | Equipment::SmallRadio | Equipment::LargeRadio | Equipment::Barrel | Equipment::Teleport => 0,
    Equipment::Napalm => 260,
    Equipment::AtomicBomb => 280,
    Equipment::MetalWall => 1,
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

fn item_placement_hits(item: Equipment) -> i32 {
  match item {
    Equipment::JumpingBomb => rand::thread_rng().gen_range(7, 27),
    Equipment::Biomass => 400,
    Equipment::Grenade => 0,
    // Note that this is also "push" difficulty and in `interact_map` we actually set it to 24
    // for pushed items (so it's easier to push for the first time). This seems to be the behavior
    // of the original game.
    _ => 20,
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
pub enum SplatterKind {
  Blood,
  Slime,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Update {
  Actor(EntityIndex, Digging),
  Map(Cursor),
  Border(Cursor),
  BurnedBorder(Cursor),
  Splatter(Cursor, Direction, SplatterKind),
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

  pub fn update_burned_border(&mut self, cursor: Cursor) {
    self.queue.push(Update::BurnedBorder(cursor));
  }

  pub fn update_splatter(&mut self, cursor: Cursor, direction: Direction, splatter: SplatterKind) {
    self.queue.push(Update::Splatter(cursor, direction, splatter));
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

/// Make a dice roll to check if fuse went out for the given bomb
fn check_fuse_went_out(value: MapValue) -> Option<MapValue> {
  let replacement = match value {
    MapValue::SmallBomb3 => MapValue::SmallBombExtinguished,
    MapValue::BigBomb3 => MapValue::BigBombExtinguished,
    MapValue::Dynamite3 => MapValue::DynamiteExtinguished,
    MapValue::Napalm1 | MapValue::Napalm2 => MapValue::NapalmExtinguished,
    _ => return None,
  };
  let mut rnd = rand::thread_rng();
  if rnd.gen_range(0, 1000) <= 10 {
    Some(replacement)
  } else {
    None
  }
}
