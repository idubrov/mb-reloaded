use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::{AnimationPhase, Digging, Glyph};
use crate::keys::Key;
use crate::settings::GameSettings;
use crate::world::actor::{ActorComponent, ActorKind};
use crate::world::equipment::Equipment;
use crate::world::map::{LevelInfo, LevelMap, MapValue, DIRT_BORDER_BITMAP, MAP_COLS, MAP_ROWS, PUSHABLE_BITMAP};
use crate::world::player::PlayerComponent;
use crate::world::position::{Cursor, Direction};
use crate::world::{EntityIndex, Maps, World};
use crate::Application;
use rand::prelude::*;
use rand::Rng;
use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::rc::Rc;

impl Application<'_> {
  /// Play game, starting from player selection
  pub fn play_game(&self, ctx: &mut ApplicationContext, settings: &GameSettings) -> Result<(), anyhow::Error> {
    sdl2::mixer::Music::halt();
    let selected = self.players_select_menu(ctx, settings.options.players)?;
    if selected.is_empty() {
      return Ok(());
    }

    let mut players = Vec::with_capacity(selected.len());
    for (idx, selected) in selected.into_iter().enumerate() {
      players.push(PlayerComponent::new(
        selected,
        settings.keys.keys[idx],
        &settings.options,
      ));
    }

    for round in 0..settings.options.rounds {
      ctx.with_render_context(|canvas| {
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        let color = self.main_menu.palette[1];
        self
          .font
          .render(canvas, 220, 200, color, "Creating level...please wait")?;
        Ok(())
      })?;

      // Generate level if necessary
      // FIXME: for single player, load fixed set of levels
      ctx.animate(Animation::FadeUp, 7)?;
      let level = settings
        .levels
        .get(usize::from(round))
        .map(Rc::as_ref)
        .unwrap_or(&LevelInfo::Random);
      ctx.animate(Animation::FadeDown, 7)?;

      if self.play_round(ctx, &mut players, round, level, settings)? {
        break;
      }
    }
    Ok(())
  }

  pub fn play_round(
    &self,
    ctx: &mut ApplicationContext,
    players: &mut [PlayerComponent],
    round: u16,
    level: &LevelInfo,
    settings: &GameSettings,
  ) -> Result<bool, anyhow::Error> {
    let darkness = settings.options.darkness || players.len() == 1;
    let level = match level {
      LevelInfo::Random => {
        let mut level = LevelMap::random_map(settings.options.treasures);
        level.generate_entrances(settings.options.players);
        level
      }
      LevelInfo::File { map, .. } => map.clone(),
    };

    // Play shop music
    if std::env::var("DEV").is_err() {
      self.music2.play(-1).map_err(SdlError)?;
      sdl2::mixer::Music::set_pos(464.8).map_err(SdlError)?;

      let mut it = players.iter_mut();
      while let Some(right) = it.next() {
        let left = it.next();
        let remaining = settings.options.rounds - round;
        let preview_map = if darkness { None } else { Some(&level) };
        self.shop(ctx, remaining, &settings.options, preview_map, left, right)?;
      }
    }

    let mut world = World::create(level, players, darkness);

    // FIXME: start playing random music from level music
    sdl2::mixer::Music::halt();

    ctx.with_render_context(|canvas| {
      self.render_game_screen(canvas, &world)?;
      Ok(())
    })?;
    ctx.animate(Animation::FadeUp, 7)?;

    let mut end_round_counter = 0;
    let mut round_counter = 0;
    loop {
      // FIXME: check if escape is pressed
      // FIXME: check if game is paused
      // FIXME: check F5 -- toggle music
      // FIXME: check F10 -- exit game

      if round_counter % 18 == 0 {
        world.update_super_drill();
      }

      if world.shake > 0 {
        world.shake -= 1;
      }

      ctx.with_render_context(|canvas| {
        self.bombs_clock(canvas, &mut world)?;
        self.atomic_shake(canvas, &mut world)?;
        if round_counter % 5 == 0 {
          if world.is_single_player() {
            if world.actors[0].is_dead {
              if end_round_counter == 0 {
                world.players[0].lives -= 1;
                // FIXME: end round
                if world.players[0].lives == 0 {
                  // FIXME: end game
                }
                self.render_lives(canvas, world.players[0].lives)?;
              } else {
                end_round_counter += 2;
              }
            }
          } else if world.alive_players() < 2 {
            end_round_counter += 3;
          }
        }

        for monster in 0..world.players.len() {
          if !world.actors[monster].is_dead {
            self.animate_actor(canvas, monster, &mut world)?;
            if world.actors[monster].super_drill_count > 0 {
              self.animate_actor(canvas, monster, &mut world)?;
            }
          }

          if round_counter % 2 == 0 {
            // FIXME: player keys
            // FIXME: check player died
          }
        }
        Ok(())
      })?;

      if world.shake % 2 != 0 {
        ctx.present_shake(world.shake)?;
      } else {
        ctx.present()?;
      }

      // Handle player commands
      if round_counter % 2 == 0 {
        // FIXME: in original game, command has slight delay on facing direction
        //  However, facing seems to be only used when holding still, so doesn't really matter much.
        for event in ctx.poll_iter() {
          if let Event::KeyDown { scancode, .. } = event {
            for player in 0..world.players.len() {
              let keys = &world.players[player].keys;
              let mut actor = &mut world.actors[player];
              if keys[Key::Up] == scancode {
                actor.facing = Direction::Up;
                actor.moving = true;
              } else if keys[Key::Down] == scancode {
                actor.facing = Direction::Down;
                actor.moving = true;
              } else if keys[Key::Left] == scancode {
                actor.facing = Direction::Left;
                actor.moving = true;
              } else if keys[Key::Right] == scancode {
                actor.facing = Direction::Right;
                actor.moving = true;
              } else if keys[Key::Bomb] == scancode {
                // FIXME: temporary
                world.shake = 10;
              }
            }
          }
        }
      }

      round_counter += 1;
      if round_counter % 20 == 0 {
        // FIXME: update remaining time indicator
      }

      std::thread::sleep(std::time::Duration::from_millis(20));
    }
  }

  fn render_game_screen(&self, canvas: &mut WindowCanvas, world: &World) -> Result<(), anyhow::Error> {
    canvas.copy(&self.players.texture, None, None).map_err(SdlError)?;

    self.render_level(canvas, &world.maps.level, world.maps.darkness)?;
    if world.maps.darkness {
      canvas.set_draw_color(Color::BLACK);
      canvas.fill_rect(Rect::new(10, 40, 620, 430)).map_err(SdlError)?;
    } else {
      // Render actors
      for actor in &world.actors {
        self.render_actor(canvas, actor)?;
      }
    }

    self.render_players_info(canvas, world)?;
    if world.is_single_player() {
      self.render_lives(canvas, world.players[0].lives)?;
    } else {
      // Time bar
      canvas.set_draw_color(self.players.palette[6]);
      canvas.fill_rect(Rect::new(2, 473, 636, 5)).map_err(SdlError)?;
    }
    Ok(())
  }

  fn render_level(&self, canvas: &mut WindowCanvas, level: &LevelMap, darkness: bool) -> Result<(), anyhow::Error> {
    let mut render = |cursor: Cursor| {
      let glyph = Glyph::Map(level[cursor]);
      let pos = cursor.position();
      self
        .glyphs
        .render(canvas, i32::from(pos.x) - 5, i32::from(pos.y) - 5, glyph)
    };
    if darkness {
      // Only render borders
      for row in 0..MAP_ROWS {
        render(Cursor::new(row, 0))?;
        render(Cursor::new(row, MAP_COLS - 1))?;
      }
      for col in 0..MAP_COLS {
        render(Cursor::new(0, col))?;
        render(Cursor::new(MAP_ROWS - 1, col))?;
      }
    } else {
      // Render everything
      for cursor in Cursor::all() {
        render(cursor)?;
      }

      // Render dirt borders
      for cursor in Cursor::all_without_borders() {
        if DIRT_BORDER_BITMAP[level[cursor]] {
          self.render_dirt_border(canvas, cursor, level)?;
        }
      }
    }
    Ok(())
  }

  /// Render smoothed border for both stone and dirt blocks
  fn render_dirt_border(
    &self,
    canvas: &mut WindowCanvas,
    cursor: Cursor,
    level: &LevelMap,
  ) -> Result<(), anyhow::Error> {
    let pos_x = i32::from(10 * cursor.col);
    let pos_y = i32::from(10 * cursor.row + 30);

    // Dirt
    for dir in Direction::all() {
      let value = level[cursor.to(dir)];
      let is_corner = match dir {
        Direction::Right if value == MapValue::StoneTopLeft || value == MapValue::StoneBottomLeft => true,
        Direction::Left if value == MapValue::StoneTopRight || value == MapValue::StoneBottomRight => true,
        Direction::Down if value == MapValue::StoneTopLeft || value == MapValue::StoneTopRight => true,
        Direction::Up if value == MapValue::StoneBottomRight || value == MapValue::StoneBottomLeft => true,
        _ => false,
      };
      if (value >= MapValue::Sand1 && value <= MapValue::HeavyGravel) || is_corner {
        let (dx, dy) = border_offset(dir);
        self
          .glyphs
          .render(canvas, pos_x + dx, pos_y + dy, Glyph::SandBorder(dir.reverse()))?;
      }
    }

    // Stone
    for dir in Direction::all() {
      let value = level[cursor.to(dir)];
      if value >= MapValue::Stone1 && value <= MapValue::Stone4 {
        let (dx, dy) = border_offset(dir);
        self
          .glyphs
          .render(canvas, pos_x + dx, pos_y + dy, Glyph::StoneBorder(dir.reverse()))?;
      }
    }
    Ok(())
  }

  fn render_players_info(&self, canvas: &mut WindowCanvas, world: &World) -> Result<(), anyhow::Error> {
    // Erase extra players
    let players_len = world.players.len() as u16;
    if players_len < 4 {
      let rect = Rect::new(i32::from(players_len) * 160, 0, u32::from(4 - players_len) * 160, 30);
      canvas.set_draw_color(Color::BLACK);
      canvas.fill_rect(rect).map_err(SdlError)?;
    }

    // Current weapon selection
    const PLAYER_X: [i32; 4] = [12, 174, 337, 500];
    let palette = &self.players.palette;
    for (idx, (player, pos_x)) in world.players.iter().zip(PLAYER_X.iter()).enumerate() {
      self
        .glyphs
        .render(canvas, *pos_x, 0, Glyph::Selection(player.selection))?;
      self.font.render(
        canvas,
        *pos_x,
        0,
        palette[1],
        &player.inventory[player.selection].to_string(),
      )?;

      //canvas.set_draw_color(Color::BLACK);
      //canvas.fill_rect(Rect::new(pos_x + 50, 11, 40, 8)).map_err(SdlError)?;
      self.font.render(
        canvas,
        pos_x + 50,
        11,
        palette[3],
        &world.actors[idx].drilling.to_string(),
      )?;
      self
        .font
        .render(canvas, pos_x + 36, 1, palette[1], &player.stats.name)?;

      //canvas.set_draw_color(Color::BLACK);
      //canvas.fill_rect(Rect::new(pos_x + 50, 21, 40, 8)).map_err(SdlError)?;
      let total_cash = player.cash + world.actors[idx].accumulated_cash;
      self
        .font
        .render(canvas, pos_x + 50, 21, palette[5], &total_cash.to_string())?;
    }

    Ok(())
  }

  fn render_lives(&self, _canvas: &mut WindowCanvas, _lives: u32) -> Result<(), anyhow::Error> {
    unimplemented!()
  }

  fn render_actor(&self, canvas: &mut WindowCanvas, actor: &ActorComponent) -> Result<(), anyhow::Error> {
    // FIXME: handle moving directions, too
    let pos_x = i32::from(actor.pos.x) - 5;
    let pos_y = i32::from(actor.pos.y) - 5;
    let glyph = Glyph::Monster(actor.kind, actor.facing, Digging::Hands, AnimationPhase::Phase1);
    self.glyphs.render(canvas, pos_x, pos_y, glyph)?;
    Ok(())
  }

  fn animate_actor(
    &self,
    canvas: &mut WindowCanvas,
    entity: EntityIndex,
    world: &mut World,
  ) -> Result<(), anyhow::Error> {
    let actor = &mut world.actors[entity];
    if !actor.moving {
      self.render_actor(canvas, actor)?;
      return Ok(());
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
    let map_value = world.maps.level[cursor.to(direction)];
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
      self.reveal_map_square(canvas, cur, &mut world.maps)?;
    }

    // We are centered in the direction we are going -- hit the map!
    if delta_dir == 5 {
      self.interact_map(canvas, entity, cursor.to(direction), world)?;
    }

    // Finishing moving from adjacent square -- render that square
    if finishing_move {
      self.reveal_map_square(canvas, cursor.to(direction.reverse()), &mut world.maps)?;
    }

    // Check if we need to show animation with pick axe or without
    let is_hard = delta_dir == 5
      && ((map_value >= MapValue::StoneTopLeft && map_value <= MapValue::StoneBottomRight)
        || map_value == MapValue::StoneBottomLeft
        || (map_value >= MapValue::Stone1 && map_value <= MapValue::Stone4)
        || (map_value >= MapValue::StoneLightCracked && map_value <= MapValue::StoneHeavyCracked)
        || (map_value >= MapValue::Brick && map_value <= MapValue::BrickHeavyCracked));
    let digging = if is_hard { Digging::Pickaxe } else { Digging::Hands };

    self.animate_digging(canvas, &mut world.actors[entity], digging)?;
    Ok(())
  }

  /// Interact with the map cell (dig it with a pickaxe, pick up gold, press buttons).
  #[allow(clippy::cognitive_complexity)]
  fn interact_map(
    &self,
    canvas: &mut WindowCanvas,
    entity: EntityIndex,
    cursor: Cursor,
    world: &mut World,
  ) -> Result<(), anyhow::Error> {
    let value = world.maps.level[cursor];
    if value.is_passable() {
      if let Some(player) = world.players.get_mut(entity) {
        player.stats.meters_ran += 1;
        if world.maps.darkness {
          self.reveal_view(world)?;
        }
      }
    }

    if value == MapValue::Passage {
      // FIXME: temporary
    } else if value == MapValue::MetalWall
      || value.is_sand()
      || value.is_stone_like()
      || value.is_brick_like()
      || value == MapValue::BioMass
      || value == MapValue::Plastic
      || value == MapValue::ExplosivePlastic
      || value == MapValue::LightGravel
      || value == MapValue::HeavyGravel
    {
      let actor = &world.actors[entity];
      // Diggable squares
      // FIXME: use mapvalueset

      if world.maps.hits[cursor] == 30_000 {
        // 30_000 is a metal wall
      } else if world.maps.hits[cursor] > 1 {
        world.maps.hits[cursor] -= i32::from(actor.drilling);
        if value.is_stone_like() {
          if world.maps.hits[cursor] < 500 {
            if value.is_stone_corner() {
              world.maps.level[cursor] = MapValue::LightGravel;
            } else {
              world.maps.level[cursor] = MapValue::StoneHeavyCracked;
            }
            self.reveal_map_square(canvas, cursor, &mut world.maps)?;
          } else if world.maps.hits[cursor] < 1000 {
            if value.is_stone_corner() {
              world.maps.level[cursor] = MapValue::HeavyGravel;
            } else {
              world.maps.level[cursor] = MapValue::StoneLightCracked;
            }
            self.reveal_map_square(canvas, cursor, &mut world.maps)?;
          }
        } else if value.is_brick_like() {
          if world.maps.hits[cursor] <= 2000 {
            world.maps.level[cursor] = MapValue::BrickHeavyCracked;
          } else if world.maps.hits[cursor] <= 4000 {
            world.maps.level[cursor] = MapValue::BrickLightCracked;
          }
          self.reveal_map_square(canvas, cursor, &mut world.maps)?;
          return Ok(());
        }
      } else {
        world.maps.hits[cursor] = 0;
        world.maps.level[cursor] = MapValue::Passage;
        self.reveal_map_square(canvas, cursor, &mut world.maps)?;
        self.render_dirt_border(canvas, cursor, &world.maps.level)?;
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
      let gold_value = match value {
        MapValue::GoldShield => 15,
        MapValue::GoldEgg => 25,
        MapValue::GoldPileCoins => 15,
        MapValue::GoldBracelet => 10,
        MapValue::GoldBar => 30,
        MapValue::GoldCross => 35,
        MapValue::GoldScepter => 50,
        MapValue::GoldRubin => 65,
        MapValue::GoldCrown => 100,
        MapValue::Diamond => 1000,
        _ => 0,
      };

      let actor = &world.actors[entity];
      if let Some(player) = actor.owner {
        world.actors[player].drilling += drill_value;
        world.actors[player].accumulated_cash += gold_value;
      }

      world.actors[entity].drilling += drill_value;
      world.actors[entity].accumulated_cash = gold_value;

      if value >= MapValue::SmallPickaxe && value <= MapValue::Drill {
        // FIXME: Play picaxe.voc, freq: 11000
      } else {
        // FIXME: play kili.voc, freq: 10000, 12599 or 14983
        if let Some(player) = world.player_mut(entity) {
          player.stats.treasures_collected += 1;
        }
      }

      // FIXME: optimized re-rendering?
      self.render_players_info(canvas, world)?;

      world.maps.hits[cursor] = 0;
      world.maps.level[cursor] = MapValue::Passage;
      self.reveal_map_square(canvas, cursor, &mut world.maps)?;
    } else if value == MapValue::Mine {
      // Activate the mine
      world.maps.timer[cursor] = 1;
    } else if PUSHABLE_BITMAP[value] {
      let actor = &world.actors[entity];
      // Go to the target position
      let target = cursor.to(actor.facing);
      if world.maps.hits[cursor] == 30_000 {
        // FIXME: wall shouldn't be pushable anyways?
      } else if world.maps.hits[cursor] > 1 {
        // Still need to push a little
        world.maps.hits[cursor] -= i32::from(actor.drilling);
      } else if world.maps.level[target].is_passable() {
        // Check if no actors are blocking the path
        if world.actors.iter().all(|p| p.is_dead || p.pos.cursor() != target) {
          // Push to `target` location
          world.maps.level[target] = world.maps.level[cursor];
          world.maps.timer[target] = world.maps.timer[cursor];
          world.maps.hits[target] = 24;

          // Clear old position
          world.maps.level[cursor] = MapValue::Passage;
          world.maps.timer[cursor] = 0;

          // FIXME: re-render blood
          reapply_blood(cursor, world);
          self.reveal_map_square(canvas, cursor, &mut world.maps)?;
          self.reveal_map_square(canvas, target, &mut world.maps)?;
        }
      }
    } else if value == MapValue::WeaponsCrate {
      // FIXME: play sound sample picaxe, freq = 11000, at column
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
          if let Some(player) = world.player_mut(entity) {
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
          if let Some(player) = world.player_mut(entity) {
            player.inventory[weapon] += cnt;
          }
        }
        _ => {
          let cnt = rng.gen_range(3, 13);
          let weapon = *[
            Equipment::SmallBomb,
            Equipment::LargeBomb,
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
          if let Some(player) = world.player_mut(entity) {
            player.inventory[weapon] += cnt;
          }
        }
      }

      world.maps.hits[cursor] = 0;
      world.maps.level[cursor] = MapValue::Passage;
      self.reveal_map_square(canvas, cursor, &mut world.maps)?;

      // FIXME: more optimal re-rendering?
      self.render_players_info(canvas, world)?;
    } else if value == MapValue::LifeItem {
      if world.actors[entity].kind == ActorKind::Player1 {
        world.players[0].lives += 1;
        self.render_players_info(canvas, world)?;
      }

      world.maps.hits[cursor] = 0;
      world.maps.level[cursor] = MapValue::Passage;
      self.reveal_map_square(canvas, cursor, &mut world.maps)?;
    } else if value == MapValue::ButtonOff {
      if world.maps.timer[cursor] <= 1 {
        open_doors(world);
      }
    } else if value == MapValue::ButtonOn {
      if world.maps.timer[cursor] <= 1 {
        close_doors(world);
      }
    } else if value == MapValue::Teleport {
      let mut entrance_idx = 0;
      let mut teleport_count = 0;
      for cur in Cursor::all() {
        if world.maps.level[cur] == MapValue::Teleport {
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
        if world.maps.level[cur] == MapValue::Teleport {
          if exit == 0 {
            // Found exit point
            let actor = &mut world.actors[entity];
            self.reveal_map_square(canvas, actor.pos.cursor(), &mut world.maps)?;
            // Move to the exit point
            actor.pos = cur.into();
            self.reveal_map_square(canvas, actor.pos.cursor(), &mut world.maps)?;
            break;
          }
          exit -= 1;
        }
      }
    } else if value == MapValue::Exit {
      // FIXME: exiting level
    } else if value == MapValue::Medikit {
      // FIXME: play sound picaxe.voc, freq = 11000

      // FIXME: check is_monster_active
      if true {
        world.actors[entity].health = world.actors[entity].max_health;
      }

      self.render_players_info(canvas, world)?;
      world.maps.level[cursor] = MapValue::Passage;
      self.reveal_map_square(canvas, cursor, &mut world.maps)?;
    }
    Ok(())
  }

  /// Reveal map based on player vision
  fn reveal_view(&self, _maps: &mut World) -> Result<(), anyhow::Error> {
    unimplemented!("reveal view")
  }

  fn animate_digging(
    &self,
    canvas: &mut WindowCanvas,
    monster: &mut ActorComponent,
    digging: Digging,
  ) -> Result<(), anyhow::Error> {
    if !monster.moving {
      return Ok(());
    }

    if monster.animation < 30 {
      let phase = match monster.animation / 5 {
        0 => AnimationPhase::Phase1,
        1 => AnimationPhase::Phase2,
        2 => AnimationPhase::Phase3,
        3 => AnimationPhase::Phase4,
        4 => AnimationPhase::Phase3,
        5 => AnimationPhase::Phase2,
        _ => unreachable!(),
      };
      let glyph = Glyph::Monster(monster.kind, monster.facing, digging, phase);
      self.glyphs.render(
        canvas,
        i32::from(monster.pos.x) - 5,
        i32::from(monster.pos.y) - 5,
        glyph,
      )?;
    } else {
      monster.animation = 0;
    }

    if digging == Digging::Pickaxe && monster.animation == 16 {
      // FIXME: frequency adjustment
      //let mut rng = rand::thread_rng();
      //let freq = rng.gen_range(11000, 11100)
      // FIXME: play sound picaxe.voc
    }
    monster.animation += 1;
    Ok(())
  }

  fn reveal_map_square(&self, canvas: &mut WindowCanvas, cursor: Cursor, maps: &mut Maps) -> Result<(), anyhow::Error> {
    let glyph = Glyph::Map(maps.level[cursor]);
    let pos = cursor.position();
    self
      .glyphs
      .render(canvas, i32::from(pos.x) - 5, i32::from(pos.y) - 5, glyph)?;
    maps.fog[cursor].reveal();
    Ok(())
  }

  fn bombs_clock(&self, canvas: &mut WindowCanvas, world: &mut World) -> Result<(), anyhow::Error> {
    for cursor in Cursor::all() {
      match world.maps.timer[cursor] {
        0 => {}
        1 => {
          world.maps.timer[cursor] = 0;
          if let Some(extinguished) = self.check_fuse_went_out(world.maps.level[cursor]) {
            world.maps.level[cursor] = extinguished;
            self.reveal_map_square(canvas, cursor, &mut world.maps)?;
          } else {
            self.explode_entity(canvas, cursor, world)?;
          }
        }
        clock => {
          world.maps.timer[cursor] = clock - 1;
          let replacement = match world.maps.level[cursor] {
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
          world.maps.level[cursor] = replacement;
          self.reveal_map_square(canvas, cursor, &mut world.maps)?;
        }
      }
    }
    Ok(())
  }

  /// Make a dice roll to check if fuse went out
  fn check_fuse_went_out(&self, value: MapValue) -> Option<MapValue> {
    let replacement = match value {
      MapValue::SmallBomb3 => MapValue::SmallBombExtinguished,
      MapValue::BigBomb3 => MapValue::BigBombExtinguished,
      MapValue::Dynamite3 => MapValue::DynamiteExtinguished,
      MapValue::Napalm1 | MapValue::Napalm2 => MapValue::NapalmExtinguished,
      _ => return None,
    };
    let mut rnd = rand::thread_rng();
    if rnd.gen_range(0, 1000) <= 10_000 {
      Some(replacement)
    } else {
      None
    }
  }

  fn explode_entity(
    &self,
    _canvas: &mut WindowCanvas,
    _cursor: Cursor,
    _world: &mut World,
  ) -> Result<(), anyhow::Error> {
    unimplemented!()
  }

  fn atomic_shake(&self, _canvas: &mut WindowCanvas, _world: &mut World) -> Result<(), anyhow::Error> {
    // FIXME: implement
    Ok(())
  }
}

fn border_offset(dir: Direction) -> (i32, i32) {
  match dir {
    Direction::Left => (-4, 0),
    Direction::Right => (10, 0),
    Direction::Up => (0, -3),
    Direction::Down => (0, 10),
  }
}

fn open_doors(_maps: &mut World) {
  unimplemented!()
}

fn close_doors(_maps: &mut World) {
  unimplemented!()
}

fn reapply_blood(cursor: Cursor, world: &mut World) {
  for actor in &world.actors {
    if actor.is_dead && actor.pos.cursor() == cursor {
      if actor.kind == ActorKind::Slime {
        world.maps.level[cursor] = MapValue::SlimeCorpse;
      } else {
        world.maps.level[cursor] = MapValue::Blood;
      }
      break;
    }
  }
}
