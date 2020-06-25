use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::{AnimationPhase, Digging, Glyph};
use crate::keys::Key;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RoundEnd {
  Round,
  Game,
}

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

      if self.play_round(ctx, &mut players, round, level, settings)? == RoundEnd::Game {
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
  ) -> Result<RoundEnd, anyhow::Error> {
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
        }
        // Take all the updates
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

        // FIXME: update round time
        world.update.queue.clear();
        Ok(())
      })?;

      if world.round_counter % 20 == 0 {
        // FIXME: update remaining time indicator
      }

      if world.is_end_of_round() {
        break RoundEnd::Round;
      }

      if world.flash {
        ctx.present_flash()?;
      } else if world.shake % 2 != 0 {
        ctx.present_shake(world.shake)?;
      } else {
        ctx.present()?;
      }
      std::thread::sleep(std::time::Duration::from_millis(20));
    };
    ctx.animate(Animation::FadeDown, 7)?;

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
        self
          .glyphs
          .render(canvas, pos_x + dx, pos_y + dy, Glyph::SandBorder(dir.reverse()))?;
      }
    }

    // Stone
    for dir in Direction::all() {
      let value = level[cursor.to(dir)];
      if value.is_stone() {
        let (dx, dy) = border_offset(dir);
        self
          .glyphs
          .render(canvas, pos_x + dx, pos_y + dy, Glyph::StoneBorder(dir.reverse()))?;
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
    if value == MapValue::Explosion || value == MapValue::MonsterExploding {
      for dir in Direction::all() {
        let value = level[cursor.to(dir)];
        let glyph = if value.is_sand() || value == MapValue::LightGravel || value == MapValue::HeavyGravel {
          Glyph::BurnedBorder(dir.reverse())
        } else if value.is_stone() || value.is_stone_corner() {
          Glyph::StoneBorder(dir.reverse())
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
        if value.is_passable() || value == MapValue::Explosion || value == MapValue::MonsterExploding {
          let (dx, dy) = border_offset(dir);
          self
            .glyphs
            .render(canvas, pos_x + dx, pos_y + dy, Glyph::BurnedBorder(dir.reverse()))?;
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

    Ok(())
  }

  fn render_lives(&self, _canvas: &mut WindowCanvas, _lives: u32) -> Result<(), anyhow::Error> {
    unimplemented!()
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
    let glyph = Glyph::Map(maps.level[cursor]);
    let pos = cursor.position();
    self
      .glyphs
      .render(canvas, i32::from(pos.x) - 5, i32::from(pos.y) - 5, glyph)?;
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
