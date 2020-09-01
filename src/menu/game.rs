use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::{AnimationPhase, Border, Digging, Glyph};
use crate::keys::Key;
use crate::menu::shop::ShopResult;
use crate::settings::GameSettings;
use crate::world::actor::ActorComponent;
use crate::world::map::{LevelInfo, LevelMap, MapValue, DIRT_BORDER_BITMAP, MAP_COLS, MAP_ROWS};
use crate::world::player::PlayerComponent;
use crate::world::position::{Cursor, Direction};
use crate::world::{Maps, SplatterKind, Update, World};
use crate::Application;
use rand::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::rc::Rc;

const SINGLE_PLAYER_ROUNDS: u16 = 15;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoundEnd {
  /// Round end (all gold collected in multiplayer, all opponents are dead, etc)
  Round,
  /// Game end (exited game, died with no more lives left)
  Game,
  /// Failed round: playing single player and died
  Failed,
}

impl Application<'_> {
  /// Play game, starting from player selection
  pub fn play_game(&self, ctx: &mut ApplicationContext, settings: &GameSettings) -> Result<(), anyhow::Error> {
    sdl2::mixer::Music::halt();
    let single_player = settings.options.players == 1;
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

    if single_player {
      // In single player, we start with 250
      players[0].cash = 250;
      players[0].lives = 3;
    }

    let mut round = 0;
    while (!single_player && round < settings.options.rounds)
      || (single_player && players[0].lives > 0 && round < SINGLE_PLAYER_ROUNDS)
    {
      ctx.with_render_context(|canvas| {
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        let color = self.main_menu.palette[1];
        self
          .font
          .render(canvas, 220, 200, color, "Creating level...please wait")?;
        Ok(())
      })?;

      // Select a level to play
      ctx.animate(Animation::FadeUp, 7)?;
      let slot;
      let level = if settings.options.players == 1 {
        slot = LevelMap::prepare_singleplayer_level(ctx.game_dir(), round)?;
        &slot
      } else {
        settings
          .levels
          .get(usize::from(round))
          .map(Rc::as_ref)
          .unwrap_or(&LevelInfo::Random)
      };
      ctx.animate(Animation::FadeDown, 7)?;
      let result = self.play_round(ctx, &mut players, round, level, settings)?;
      if single_player && players[0].lives == 0 {
        // End of game: out of lives!
        break;
      }
      match result {
        RoundEnd::Game => break,
        RoundEnd::Failed => {
          // Keep playing the same round!
        }
        RoundEnd::Round => {
          round += 1;
        }
      }
    }

    if single_player {
      let texture = if round == SINGLE_PLAYER_ROUNDS {
        &self.game_win.texture
      } else {
        &self.game_over.texture
      };
      ctx.with_render_context(|canvas| {
        canvas.copy(texture, None, None).map_err(SdlError)?;
        Ok(())
      })?;
      ctx.animate(Animation::FadeUp, 7)?;
      ctx.wait_key_pressed();
      ctx.animate(Animation::FadeDown, 7)?;
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
  ) -> Result<RoundEnd, anyhow::Error> {
    // Note: in original game, single player is always played dark. However, in this
    // re-implementation I'm relaxing this as I never had patience to play through all 15 levels
    // with darkness 😅
    let darkness = settings.options.darkness; // || players.len() == 1;
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
        if self.shop(ctx, remaining, &settings.options, preview_map, left, right)? == ShopResult::ExitGame {
          return Ok(RoundEnd::Game);
        }
      }
    }

    let mut world = World::create(level, players, darkness, settings.options.bomb_damage);

    // FIXME: start playing random music from level music
    sdl2::mixer::Music::halt();

    ctx.with_render_context(|canvas| {
      self.render_game_screen(canvas, &world)?;
      Ok(())
    })?;
    ctx.animate(Animation::FadeUp, 7)?;

    let exit_reason = 'round: loop {
      world.tick();

      // Handle player commands
      if world.round_counter % 2 == 0 {
        // FIXME: in original game, command has slight delay on facing direction
        //  However, facing seems to be only used when holding still, so doesn't really matter much.

        let mut paused = false;
        for event in ctx.poll_iter() {
          if let Event::KeyDown {
            scancode: Some(scancode),
            ..
          } = event
          {
            match scancode {
              Scancode::Escape if world.is_single_player() => {
                // Artificial death
                world.players[0].lives -= 1;
                break 'round RoundEnd::Failed;
              }
              Scancode::Escape => break 'round RoundEnd::Round,
              Scancode::F10 => break 'round RoundEnd::Game,
              // FIXME: some better scancode?
              Scancode::Pause => {
                paused = true;
              }
              Scancode::F5 => unimplemented!("toggle music"),
              _ => {}
            }

            for player in 0..world.players.len() {
              let keys = world.players[player].keys;
              for key in Key::all_keys() {
                if keys[key] == Some(scancode) {
                  world.player_action(player, key);
                }
              }
            }
          }
        }
        if paused {
          ctx.wait_key_pressed();
        }
      }

      // Apply all rendering updates
      ctx.with_render_context(|canvas| {
        if world.update.players_info {
          self.render_players_info(canvas, &world)?;
          if world.is_single_player() {
            self.render_lives(canvas, world.players[0].lives)?;
          }
        }

        // Go through each update and render it
        for update in &world.update.queue {
          match *update {
            Update::Actor(actor, digging) => {
              let actor = &world.actors[actor];
              self.render_actor(canvas, actor, digging)?;
            }
            Update::Map(cursor) => {
              self.reveal_map_square(canvas, cursor, &mut world.maps)?;
            }
            Update::Border(cursor) => {
              self.render_dirt_border(canvas, cursor, &world.maps.level)?;
            }
            Update::BurnedBorder(cursor) => {
              self.render_burned_border(canvas, cursor, &world.maps.level)?;
            }
            Update::Splatter(cursor, dir, splatter) => {
              self.render_splatter(canvas, cursor, dir, splatter)?;
            }
          }
        }

        world.update.queue.clear();
        Ok(())
      })?;

      if world.round_counter % 20 == 0 {
        // FIXME: update remaining time indicator
      }

      if world.is_end_of_round() {
        if world.is_single_player() && world.actors[0].is_dead {
          break RoundEnd::Failed;
        }
        break RoundEnd::Round;
      }

      if world.flash {
        ctx.present_flash()?;
      } else if world.shake % 2 != 0 {
        ctx.present_shake(world.shake)?;
      } else {
        ctx.present()?;
      }

      // Play sound effects
      for request in &world.effects.queue {
        self.effects.play(request.effect, request.frequency, request.location)?;
      }
      world.effects.queue.clear();

      std::thread::sleep(std::time::Duration::from_millis(20));
    };
    ctx.animate(Animation::FadeDown, 7)?;

    if exit_reason == RoundEnd::Round {
      world.end_of_round();
    }

    Ok(exit_reason)
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
        self.render_actor(canvas, actor, Digging::Hands)?;
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
    let pos = cursor.position();
    let pos_x = i32::from(pos.x);
    let pos_y = i32::from(pos.y);

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
        self.glyphs.render(
          canvas,
          pos_x + dx,
          pos_y + dy,
          Glyph::SandBorder(dir.reverse(), Border::Normal),
        )?;
      }
    }

    // Stone
    for dir in Direction::all() {
      let value = level[cursor.to(dir)];
      if value.is_stone() {
        let (dx, dy) = border_offset(dir);
        self.glyphs.render(
          canvas,
          pos_x + dx,
          pos_y + dy,
          Glyph::StoneBorder(dir.reverse(), Border::Normal),
        )?;
      }
    }
    Ok(())
  }

  /// Render burned border for both stone and dirt blocks
  fn render_burned_border(
    &self,
    canvas: &mut WindowCanvas,
    cursor: Cursor,
    level: &LevelMap,
  ) -> Result<(), anyhow::Error> {
    let pos = cursor.position();
    let pos_x = i32::from(pos.x);
    let pos_y = i32::from(pos.y);

    let value = level[cursor];
    if value == MapValue::Explosion || value == MapValue::MonsterDying {
      for dir in Direction::all() {
        let value = level[cursor.to(dir)];
        let glyph = if value.is_sand() || value == MapValue::LightGravel || value == MapValue::HeavyGravel {
          Glyph::SandBorder(dir.reverse(), Border::Burned)
        } else if value.is_stone() || value.is_stone_corner() {
          Glyph::StoneBorder(dir.reverse(), Border::Burned)
        } else {
          continue;
        };
        let (dx, dy) = border_offset(dir);
        self.glyphs.render(canvas, pos_x + dx, pos_y + dy, glyph)?;
      }
    } else if value == MapValue::HeavyGravel {
      // FIXME: not sure when this one is triggered?
      for dir in Direction::all() {
        let value = level[cursor.to(dir)];
        if value.is_passable() || value == MapValue::Explosion || value == MapValue::MonsterDying {
          let (dx, dy) = border_offset(dir);
          self.glyphs.render(
            canvas,
            pos_x + dx,
            pos_y + dy,
            Glyph::SandBorder(dir.reverse(), Border::Burned),
          )?;
        }
      }
    }
    Ok(())
  }

  fn render_splatter(
    &self,
    canvas: &mut WindowCanvas,
    cursor: Cursor,
    dir: Direction,
    splatter: SplatterKind,
  ) -> Result<(), anyhow::Error> {
    let mut rng = rand::thread_rng();
    let color = match splatter {
      SplatterKind::Blood => 3,
      SplatterKind::Slime => 4,
    };
    canvas.set_draw_color(self.players.palette[color]);
    let pos = cursor.position();
    loop {
      let (delta_x, delta_y) = match dir {
        Direction::Left => (-5 - rng.gen_range(0, 3), rng.gen_range(-5, 5)),
        Direction::Right => (5 + rng.gen_range(0, 3), rng.gen_range(-5, 5)),
        Direction::Up => (rng.gen_range(-5, 5), -5 - rng.gen_range(0, 3)),
        Direction::Down => (rng.gen_range(-5, 5), 5 + rng.gen_range(0, 3)),
      };
      canvas
        .draw_point((i32::from(pos.x) + delta_x, i32::from(pos.y) + delta_y))
        .map_err(SdlError)?;
      if rng.gen_range(0, 10) == 0 {
        break;
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

      canvas.set_draw_color(Color::BLACK);
      canvas.fill_rect(Rect::new(pos_x + 50, 11, 40, 8)).map_err(SdlError)?;
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

      canvas.set_draw_color(Color::BLACK);
      canvas.fill_rect(Rect::new(pos_x + 50, 21, 40, 8)).map_err(SdlError)?;
      let total_cash = player.cash + world.actors[idx].accumulated_cash;
      self
        .font
        .render(canvas, pos_x + 50, 21, palette[5], &total_cash.to_string())?;
    }

    // Players health
    const HEALTH_COLOR: [usize; 4] = [2, 3, 4, 6];
    const HEALTH_BAR_LEFT: [i32; 4] = [142, 304, 467, 630];
    for player in 0..world.players.len() {
      let actor = &world.actors[player];
      let health_bars = if actor.health == 0 {
        0
      } else {
        (u32::from(actor.health) * 50 + 1) / (2 * u32::from(actor.max_health)) + 1
      };
      let left = HEALTH_BAR_LEFT[player];
      canvas.set_draw_color(Color::BLACK);
      if health_bars < 25 {
        canvas
          .fill_rect(Rect::new(left, 2, 8, 26 - health_bars))
          .map_err(SdlError)?;
      }
      if health_bars > 0 {
        canvas.set_draw_color(palette[HEALTH_COLOR[player]]);
        canvas
          .fill_rect(Rect::new(left, 28 - (health_bars as i32), 8, health_bars))
          .map_err(SdlError)?;
      }
    }
    Ok(())
  }

  fn render_lives(&self, canvas: &mut WindowCanvas, lives: u16) -> Result<(), anyhow::Error> {
    canvas.set_draw_color(Color::BLACK);
    canvas.fill_rect(Rect::new(160, 2, 480, 28)).map_err(SdlError)?;
    for idx in 0..lives.max(3) {
      let glyph = if idx < lives { Glyph::Life } else { Glyph::LifeLost };
      self.glyphs.render(canvas, i32::from(idx * 16) + 160, 2, glyph)?;
    }
    Ok(())
  }

  fn render_actor(
    &self,
    canvas: &mut WindowCanvas,
    actor: &ActorComponent,
    digging: Digging,
  ) -> Result<(), anyhow::Error> {
    let phase = match actor.animation / 5 {
      _ if !actor.moving => AnimationPhase::Phase1,
      0 => AnimationPhase::Phase1,
      1 => AnimationPhase::Phase2,
      2 => AnimationPhase::Phase3,
      3 => AnimationPhase::Phase4,
      4 => AnimationPhase::Phase3,
      _ => AnimationPhase::Phase2,
    };

    let pos_x = i32::from(actor.pos.x) - 5;
    let pos_y = i32::from(actor.pos.y) - 5;
    let glyph = Glyph::Monster(actor.kind, actor.facing, digging, phase);
    self.glyphs.render(canvas, pos_x, pos_y, glyph)?;
    Ok(())
  }

  fn reveal_map_square(&self, canvas: &mut WindowCanvas, cursor: Cursor, maps: &mut Maps) -> Result<(), anyhow::Error> {
    // FIXME: temporary. Need to figure out what to do with time bar
    if cursor.row == MAP_ROWS - 1 {
      return Ok(());
    }

    let glyph = Glyph::Map(maps.level[cursor]);
    let pos = cursor.position();
    self
      .glyphs
      .render(canvas, i32::from(pos.x) - 5, i32::from(pos.y) - 5, glyph)?;
    // FIXME: move to world?
    maps.fog[cursor].reveal();
    Ok(())
  }
}

fn border_offset(dir: Direction) -> (i32, i32) {
  match dir {
    Direction::Left => (-9, -5),
    Direction::Right => (5, -5),
    Direction::Up => (-5, -8),
    Direction::Down => (-5, 5),
  }
}
