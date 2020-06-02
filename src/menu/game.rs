use crate::context::{Animation, ApplicationContext};
use crate::entity::{Direction, Equipment, MonsterEntity, PlayerEntity};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::map::{FogMap, HitsMap, LevelInfo, LevelMap, MapValue, TimerMap, MAP_COLS, MAP_ROWS};
use crate::settings::GameSettings;
use crate::Application;
use rand::Rng;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::rc::Rc;

struct RoundState<'p> {
  darkness: bool,
  #[allow(dead_code)]
  timer: TimerMap,
  level: LevelMap,
  #[allow(dead_code)]
  hits: HitsMap,
  #[allow(dead_code)]
  fog: FogMap,
  #[allow(dead_code)]
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
    let state = RoundState {
      darkness: settings.options.darkness,
      timer: TimerMap::from_level_map(&level),
      hits: HitsMap::from_level_map(&level),
      fog: FogMap::new(),
      level,
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
        Some(&state.level)
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
    Ok(true)
  }

  fn render_game_screen(&self, canvas: &mut WindowCanvas, state: &RoundState) -> Result<(), anyhow::Error> {
    canvas.copy(&self.players.texture, None, None).map_err(SdlError)?;

    self.render_level(canvas, &state.level, state.darkness)?;
    if state.darkness {
      canvas.set_draw_color(Color::BLACK);
      canvas.fill_rect(Rect::new(10, 40, 620, 430)).map_err(SdlError)?;
    }

    // Erase extra players
    let players = state.players.len() as u16;
    if players < 4 {
      let rect = Rect::new(i32::from(players) * 160, 0, u32::from(4 - players) * 160, 30);
      canvas.set_draw_color(Color::BLACK);
      canvas.fill_rect(rect).map_err(SdlError)?;
    }

    // FIXME: render player selection
    // FIXME: render drilling power
    // FIXME: render player names
    // FIXME: render cash
    // FIXME: render selected item count

    if players == 1 {
      // FIXME: render lives
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
          let value = level[row][col] as u8;
          if DIRT_BORDER_BITMAP[usize::from(value / 8)] & (1 << (value & 7)) != 0 {
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
      let value = level.cursor(row, col)[dir];
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
      let value = level.cursor(row, col)[dir];
      if value >= MapValue::Stone1 && value <= MapValue::Stone4 {
        let (dx, dy) = border_offset(dir);
        self
          .glyphs
          .render(canvas, pos_x + dx, pos_y + dy, Glyph::StoneBorder(dir.reverse()))?;
      }
    }
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

/// Bitmap of which map values are exposing border of surrounding dirt and stones.
const DIRT_BORDER_BITMAP: [u8; 32] = [
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0001,
  0b0000_0000,
  0b0000_0100,
  0b0000_0000,
  0b1000_0000,
  0b1000_0011,
  0b1111_1000,
  0b0011_1111,
  0b1000_1000,
  0b1111_0011,
  0b0000_1111,
  0b1111_1100,
  0b1111_1111,
  0b1111_0111,
  0b1111_1111,
  0b1000_1111,
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
];
