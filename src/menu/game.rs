use crate::context::{Animation, ApplicationContext};
use crate::entity::{Direction, Equipment, MonsterEntity, PlayerEntity};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::map::bitmaps::DIRT_BORDER_BITMAP;
use crate::map::{Cursor, FogMap, HitsMap, LevelInfo, LevelMap, MapValue, TimerMap, MAP_COLS, MAP_ROWS};
use crate::settings::GameSettings;
use crate::Application;
use rand::Rng;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::rc::Rc;

struct Maps {
  #[allow(dead_code)]
  timer: TimerMap,
  level: LevelMap,
  hits: HitsMap,
  fog: FogMap,
}

struct RoundState<'p> {
  darkness: bool,
  maps: Maps,
  monsters: Vec<MonsterEntity>,
  players: &'p mut [PlayerEntity],
}

impl Application<'_> {
  /// Play game, starting from player selection
  pub fn play_game(&self, ctx: &mut ApplicationContext, settings: &GameSettings) -> Result<(), anyhow::Error> {
    sdl2::mixer::Music::halt();
    let selected = self.players_select_menu(ctx, settings.options.players)?;
    if selected.is_empty() {
      return Ok(());
    }

    let mut entities = Vec::with_capacity(selected.len());
    for (idx, selected) in selected.into_iter().enumerate() {
      entities.push(PlayerEntity::new(
        selected,
        settings.keys.keys[idx],
        u32::from(settings.options.cash),
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

      if self.play_round(ctx, &mut entities, round, level, settings)? {
        break;
      }
    }
    Ok(())
  }

  pub fn play_round(
    &self,
    ctx: &mut ApplicationContext,
    players: &mut [PlayerEntity],
    round: u16,
    level: &LevelInfo,
    settings: &GameSettings,
  ) -> Result<bool, anyhow::Error> {
    let mut level = match level {
      LevelInfo::Random => {
        let mut level = LevelMap::random_map(settings.options.treasures);
        level.generate_entrances(settings.options.players);
        level
      }
      LevelInfo::File { map, .. } => map.clone(),
    };

    let monsters = MonsterEntity::from_map(&mut level);
    let mut state = RoundState {
      darkness: settings.options.darkness,
      maps: Maps {
        timer: TimerMap::from_level_map(&level),
        hits: HitsMap::from_level_map(&level),
        fog: FogMap::new(),
        level,
      },
      monsters,
      players,
    };
    for player in state.players.iter_mut() {
      player.inventory[Equipment::Armor] = 0;
      player.accumulated_cash = 0;
      // FIXME: facing_direction = 0
      // FIXME: direction = 1
      // FIXME: animation_clock = 1
      // FIXME: field_21 = 0
    }
    init_players_positions(state.players);

    // Play shop music
    self.music2.play(-1).map_err(SdlError)?;
    sdl2::mixer::Music::set_pos(464.8).map_err(SdlError)?;

    let mut it = state.players.iter_mut();
    while let Some(right) = it.next() {
      let left = it.next();
      let remaining = settings.options.rounds - round;
      let preview_map = if settings.options.darkness {
        None
      } else {
        Some(&state.maps.level)
      };
      self.shop(ctx, remaining, &settings.options, preview_map, left, right)?;
    }

    // FIXME: start playing random music from level music
    sdl2::mixer::Music::halt();

    ctx.with_render_context(|canvas| {
      self.render_game_screen(canvas, &state)?;
      Ok(())
    })?;
    ctx.animate(Animation::FadeUp, 7)?;
    ctx.wait_key_pressed();

    // FIXME: dev!
    if true {
      for monster in &mut state.monsters {
        monster.moving = Some(monster.facing);
      }

      loop {
        ctx.with_render_context(|canvas| {
          for monster in &mut state.monsters {
            self.animate_monster(canvas, monster, &mut state.maps)?;
          }
          Ok(())
        })?;
        ctx.present()?;
      }
    }
    Ok(true)
  }

  fn render_game_screen(&self, canvas: &mut WindowCanvas, state: &RoundState) -> Result<(), anyhow::Error> {
    canvas.copy(&self.players.texture, None, None).map_err(SdlError)?;

    self.render_level(canvas, &state.maps.level, state.darkness)?;
    if state.darkness {
      canvas.set_draw_color(Color::BLACK);
      canvas.fill_rect(Rect::new(10, 40, 620, 430)).map_err(SdlError)?;
    } else {
      // Render monsters
      for monster in &state.monsters {
        self.render_monster(canvas, monster)?;
      }
    }

    self.render_players_info(canvas, state.players)?;
    if state.players.len() == 1 {
      unimplemented!("render lives");
    } else {
      // Time bar
      canvas.set_draw_color(self.players.palette[6]);
      canvas.fill_rect(Rect::new(2, 473, 636, 5)).map_err(SdlError)?;
    }
    Ok(())
  }

  fn render_level(&self, canvas: &mut WindowCanvas, level: &LevelMap, darkness: bool) -> Result<(), anyhow::Error> {
    let mut render = |row: usize, col: usize| {
      let glyph = Glyph::Map(level[row][col]);
      self
        .glyphs
        .render(canvas, (col * 10) as i32, (row * 10 + 30) as i32, glyph)
    };
    if darkness {
      // Only render borders
      for row in 0..MAP_ROWS {
        render(row, 0)?;
        render(row, MAP_COLS - 1)?;
      }
      for col in 0..MAP_COLS {
        render(0, col)?;
        render(MAP_ROWS - 1, col)?;
      }
    } else {
      // Render everything
      for row in 0..MAP_ROWS {
        for col in 0..MAP_COLS {
          render(row, col)?;
        }
      }

      // Render dirt borders
      for row in 1..MAP_ROWS - 1 {
        for col in 1..MAP_COLS - 1 {
          if DIRT_BORDER_BITMAP[level[row][col] as usize] {
            self.render_dirt_border(canvas, level, row, col)?;
          }
        }
      }
    }
    Ok(())
  }

  /// Render smoothed border for both stone and dirt blocks
  fn render_dirt_border(
    &self,
    canvas: &mut WindowCanvas,
    level: &LevelMap,
    row: usize,
    col: usize,
  ) -> Result<(), anyhow::Error> {
    let pos_x = (10 * col) as i32;
    let pos_y = (10 * row + 30) as i32;

    // Dirt
    for dir in Direction::all() {
      let value = level[Cursor::new(row, col).to(dir)];
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
      let value = level[Cursor::new(row, col).to(dir)];
      if value >= MapValue::Stone1 && value <= MapValue::Stone4 {
        let (dx, dy) = border_offset(dir);
        self
          .glyphs
          .render(canvas, pos_x + dx, pos_y + dy, Glyph::StoneBorder(dir.reverse()))?;
      }
    }
    Ok(())
  }

  fn render_players_info(&self, canvas: &mut WindowCanvas, players: &[PlayerEntity]) -> Result<(), anyhow::Error> {
    // Erase extra players
    let players_len = players.len() as u16;
    if players_len < 4 {
      let rect = Rect::new(i32::from(players_len) * 160, 0, u32::from(4 - players_len) * 160, 30);
      canvas.set_draw_color(Color::BLACK);
      canvas.fill_rect(rect).map_err(SdlError)?;
    }

    // Current weapon selection
    const PLAYER_X: [i32; 4] = [12, 174, 337, 500];
    let palette = &self.players.palette;
    for (player, pos_x) in players.iter().zip(PLAYER_X.iter()) {
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
        &format!("{}", player.drilling_power()),
      )?;
      self
        .font
        .render(canvas, pos_x + 36, 1, palette[1], &player.player.name)?;

      //canvas.set_draw_color(Color::BLACK);
      //canvas.fill_rect(Rect::new(pos_x + 50, 21, 40, 8)).map_err(SdlError)?;
      self
        .font
        .render(canvas, pos_x + 50, 21, palette[5], &player.cash().to_string())?;
    }

    Ok(())
  }

  fn render_monster(&self, canvas: &mut WindowCanvas, monster: &MonsterEntity) -> Result<(), anyhow::Error> {
    // FIXME: handle moving directions, too
    let pos_x = monster.pos.x - 5;
    let pos_y = monster.pos.y - 5;
    self
      .glyphs
      .render(canvas, pos_x, pos_y, Glyph::Monster(monster.kind, monster.facing, 0))?;
    Ok(())
  }

  fn animate_monster(
    &self,
    canvas: &mut WindowCanvas,
    monster: &mut MonsterEntity,
    maps: &mut Maps,
  ) -> Result<(), anyhow::Error> {
    if let Some(direction) = monster.moving {
      let row = ((monster.pos.y - 30) / 10) as usize;
      let col = (monster.pos.x / 10) as usize;
      let delta_x = monster.pos.x % 10;
      let delta_y = monster.pos.y % 10;
      let cursor = Cursor::new(row as usize, col as usize);

      let (delta_dir, delta_orthogonal, finishing_move, can_move) = match direction {
        Direction::Left => (delta_x, delta_y, delta_x > 5, monster.pos.x > 5),
        Direction::Right => (delta_x, delta_y, delta_x < 5, monster.pos.x < 635),
        Direction::Up => (delta_y, delta_x, delta_y > 5, monster.pos.y > 35),
        Direction::Down => (delta_y, delta_x, delta_y < 5, monster.pos.x < 475),
      };

      // Vertically centered enough to be moving in the current direction
      let is_moving = can_move && delta_orthogonal > 3 && delta_orthogonal < 6;
      let map_value = maps.level[cursor.to(direction)];
      // Either finishing move into the cell or cell to the left is passable
      if is_moving
        && (finishing_move
          || map_value == MapValue::Passage
          || map_value == MapValue::Blood
          || map_value == MapValue::SlimeCorpse)
      {
        monster.pos.step(direction);
      }

      if delta_orthogonal != 5 {
        // Center our position in orthogonal direction
        monster.pos.center_orthogonal(direction);

        // Need to redraw cell orthogonal to the moving direction if we are re-centering.
        let cur = match direction {
          Direction::Left | Direction::Right if delta_orthogonal > 5 => cursor.to(Direction::Down),
          Direction::Left | Direction::Right => cursor.to(Direction::Up),
          Direction::Up | Direction::Down if delta_orthogonal > 5 => cursor.to(Direction::Right),
          Direction::Up | Direction::Down => cursor.to(Direction::Left),
        };
        self.reveal_map_square(canvas, cur, &maps.level, &mut maps.fog)?;
      }

      // We are centered in the direction we are going -- hit the map!
      if delta_dir == 5 {
        self.hit_map(monster, cursor.to(direction), &mut maps.hits);
      }

      // Finishing moving from adjacent square -- render that square
      if finishing_move {
        self.reveal_map_square(canvas, cursor.to(direction.reverse()), &maps.level, &mut maps.fog)?;
      }

      // Check if we need to show animation with pick axe or without
      let pickaxe = delta_dir == 5
        && ((map_value >= MapValue::StoneTopLeft && map_value <= MapValue::StoneBottomRight)
          || map_value == MapValue::StoneBottomLeft
          || (map_value >= MapValue::Stone1 && map_value <= MapValue::Stone4)
          || (map_value >= MapValue::StoneLightCracked && map_value <= MapValue::StoneHeavyCracked)
          || (map_value >= MapValue::Brick && map_value <= MapValue::BrickHeavyCracked));

      self.animate_digging(canvas, monster, pickaxe)?;
    } else {
      self.render_monster(canvas, monster)?;
    }
    Ok(())
  }

  fn hit_map(&self, _monster: &MonsterEntity, _cursor: Cursor, _map: &mut HitsMap) {
    unimplemented!()
  }

  fn animate_digging(
    &self,
    _canvas: &mut WindowCanvas,
    _monster: &MonsterEntity,
    _pickaxe: bool,
  ) -> Result<(), anyhow::Error> {
    unimplemented!()
  }

  fn reveal_map_square(
    &self,
    canvas: &mut WindowCanvas,
    cursor: Cursor,
    level: &LevelMap,
    fog: &mut FogMap,
  ) -> Result<(), anyhow::Error> {
    let glyph = Glyph::Map(level[cursor]);
    self
      .glyphs
      .render(canvas, (cursor.col * 10) as i32, (cursor.row * 10 + 30) as i32, glyph)?;
    fog[cursor].reveal();
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

fn init_players_positions(players: &mut [PlayerEntity]) {
  let mut rng = rand::thread_rng();

  if players.len() == 1 {
    players[0].pos = (15, 45);
  } else {
    let mut rng = rand::thread_rng();

    if rng.gen::<bool>() {
      players[0].pos = (15, 45);
      players[1].pos = (625, 465);
    } else {
      players[0].pos = (625, 465);
      players[1].pos = (15, 45);
    }
  }

  if players.len() == 3 {
    if rng.gen::<bool>() {
      players[2].pos = (15, 465);
    } else {
      players[2].pos = (625, 45);
    }
  } else if players.len() == 4 {
    if rng.gen::<bool>() {
      players[2].pos = (15, 465);
      players[3].pos = (625, 45);
    } else {
      players[2].pos = (625, 45);
      players[3].pos = (15, 465);
    }
  }
}
