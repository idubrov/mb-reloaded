use crate::world::actor::{ActorComponent, ActorKind, Player};
use crate::world::map::{LevelMap, MapValue};
use crate::world::position::{Cursor, Direction};
use crate::world::{grenade_value, EntityIndex, World};
use rand::prelude::*;

impl World<'_> {
  /// Animate non-player actors
  pub(super) fn animate_monsters(&mut self) {
    let remaining_gold = self.gold_remaining();
    for actor_idx in self.players.len()..self.actors.len() {
      let monster = &self.actors[actor_idx];
      let monster_kind = monster.kind;
      let monster_cursor = monster.pos.cursor();
      if !monster.is_active || monster.is_dead {
        // Monster is not active or dead
        continue;
      }

      self.damage_players(actor_idx);

      if self.round_counter % monster_kind.speed() != 0 {
        self.animate_actor(actor_idx);
      }

      // FIXME: potentially, big difference with original game.
      // They keep separate "current direction" and "next command direction" and we keep "facing"
      // and "moving flag". This location would be the point they copy monster "command" to "current"
      // direction. We also have to set `moving` to `false`/`true` in few places to account for
      // differences (in original game, setting "next command" direction to 0 will stop actor).

      if self.round_counter % 26 == 0 {
        if let Some(bomb_cursor) = look_for_bombs(monster_cursor, &self.maps.level) {
          self.actors[actor_idx].avoid_position(bomb_cursor, &self.maps.level);
        } else {
          match look_for_players(monster_cursor, &self.actors[0..self.players.len()]) {
            // Clones shouldn't chase their player!
            Some((player_cursor, player_idx)) if self.clone_can_chase(monster_kind, player_idx) => {
              self.actors[actor_idx].head_to_target(player_cursor, &self.maps.level);

              if let ActorKind::Clone(_) = monster_kind {
                // Clones throw grenades only when actually locked on somebody
                self.grenadier_maybe_toss_grenade(actor_idx);
              }
            }
            _ if remaining_gold > 0 => {
              if let ActorKind::Clone(_) = monster_kind {
                // Clones look for gold!
                if let Some(gold_cursor) = look_for_gold(monster_cursor, &self.maps.level) {
                  self.actors[actor_idx].head_to_target(gold_cursor, &self.maps.level);
                }
              }
            }
            _ => {}
          }

          // Grenadiers always throw grenades (unless avoiding bombs)
          if monster_kind == ActorKind::Grenadier {
            self.grenadier_maybe_toss_grenade(actor_idx);
          }
        }
      }

      let actor = &self.actors[actor_idx];
      if (self.round_counter % 33 == 0 && !actor.can_move(&self.maps.level)) || self.round_counter % 121 == 0 {
        let mut rng = rand::thread_rng();
        let dir = *[Direction::Left, Direction::Right, Direction::Up, Direction::Down]
          .choose(&mut rng)
          .unwrap();
        self.actors[actor_idx].moving = true;
        self.actors[actor_idx].facing = dir;
      }
    }
  }

  fn clone_can_chase(&self, monster_kind: ActorKind, target_player: Player) -> bool {
    match monster_kind {
      ActorKind::Clone(clone_player) if clone_player != target_player && !self.campaign_mode => true,
      ActorKind::Clone(_) => false,
      _ => true,
    }
  }

  /// Make given actor to cause damage to all players in the same cell
  fn damage_players(&mut self, actor: EntityIndex) {
    let cursor = self.actors[actor].pos.cursor();
    let monster_kind = self.actors[actor].kind;
    for player_idx in 0..self.players.len() {
      let player = &mut self.actors[player_idx];
      if player.pos.cursor() == cursor {
        match (player.kind, monster_kind) {
          (ActorKind::Player(p1), ActorKind::Clone(p2)) if p1 == p2 || self.campaign_mode => {
            // Nothing! This is our clone! Also, no damage in campaign mode.
          }
          _ => {
            player.health = player.health.saturating_sub(monster_kind.damage());
            self.update.update_player_health(player_idx);
          }
        }
      }
    }
  }

  /// Throw grenades if not blocked by map or by other monster
  fn grenadier_maybe_toss_grenade(&mut self, actor: EntityIndex) {
    // Minimum distance to obstacle when grenadier still wants to throw a grenade
    const MIN_OBSTACLE_DISTANCE: i32 = 4;

    let actor = &self.actors[actor];
    if self.check_obstacle_distance(actor) > MIN_OBSTACLE_DISTANCE {
      let cursor = actor.pos.cursor();
      for player in &self.actors[..self.players.len()] {
        let player_cursor = player.pos.cursor();
        let same_row = player_cursor.row == cursor.row;
        let same_col = player_cursor.col == cursor.col;
        if same_row != same_col {
          self.maps.level[cursor] = grenade_value(actor.facing);
          self.maps.timer[cursor] = 1;
        }
      }
    }
  }

  fn check_obstacle_distance(&self, actor: &ActorComponent) -> i32 {
    const MAX_SCAN_DISTANCE: i32 = 10;

    let mut cursor = actor.pos.cursor();
    for distance in 0..MAX_SCAN_DISTANCE {
      cursor = cursor.to(actor.facing);
      // In original game, map square next to grenadier would be considered "distance = 0"
      // However, monster sitting in the next square will be "distance = 1".
      // For simplicity, we treat both the same way
      if !self.maps.level[cursor].is_passable() {
        return distance;
      }

      // Some monster is blocking grenade throw
      if self.actors[self.players.len()..]
        .iter()
        .any(|actor| actor.pos.cursor() == cursor)
      {
        return distance;
      }

      match actor.kind {
        ActorKind::Clone(player) if self.actors[player as usize].pos.cursor() == cursor => {
          return distance;
        }
        _ => {}
      }
    }
    MAX_SCAN_DISTANCE
  }
}

/// Look around for bombs
fn look_for_bombs(cursor: Cursor, level: &LevelMap) -> Option<Cursor> {
  look_around(cursor, 5, |offset| {
    let value = level[offset];
    if value.is_bomb() {
      return Some(offset);
    }
    None
  })
}

/// Look around for players
fn look_for_players(cursor: Cursor, players: &[ActorComponent]) -> Option<(Cursor, Player)> {
  look_around(cursor, 10, |offset| {
    for player in players {
      if player.pos.cursor() == offset {
        if let ActorKind::Player(player_idx) = player.kind {
          return Some((player.pos.cursor(), player_idx));
        } else {
          unreachable!();
        }
      }
    }
    None
  })
}

/// Look around for gold
fn look_for_gold(cursor: Cursor, level: &LevelMap) -> Option<Cursor> {
  look_around(cursor, 63, |offset| {
    let value = level[offset];
    if value.is_treasure()
      || value == MapValue::SmallPickaxe
      || value == MapValue::LargePickaxe
      || value == MapValue::Drill
    {
      return Some(offset);
    }
    None
  })
}

/// Look around for players
fn look_around<T>(cursor: Cursor, distance: i16, check_location: impl Fn(Cursor) -> Option<T>) -> Option<T> {
  for distance in 1..=distance {
    for dir in &[Direction::Up, Direction::Down, Direction::Left, Direction::Right] {
      for idx in -distance..=distance {
        let offset = match dir {
          Direction::Up => cursor.offset(-distance, idx),
          Direction::Down => cursor.offset(distance, idx),
          Direction::Left => cursor.offset(idx, -distance),
          Direction::Right => cursor.offset(idx, distance),
        };

        if let Some(target) = offset.and_then(&check_location) {
          return Some(target);
        }
      }
    }
  }
  None
}
