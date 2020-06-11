use crate::context::{Animation, ApplicationContext};
use crate::entity::{Direction, Equipment, MonsterEntity, MonsterKind, PlayerEntity, Position};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::{AnimationPhase, Digging, Glyph};
use crate::map::bitmaps::{DIRT_BORDER_BITMAP, PUSHABLE_BITMAP};
use crate::map::{Cursor, FogMap, HitsMap, LevelInfo, LevelMap, MapValue, TimerMap, MAP_COLS, MAP_ROWS};
use crate::settings::GameSettings;
use crate::Application;
use rand::prelude::*;
use rand::Rng;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::rc::Rc;

struct Maps {
  darkness: bool,
  #[allow(dead_code)]
  timer: TimerMap,
  level: LevelMap,
  hits: HitsMap,
  fog: FogMap,
}

struct RoundState<'p> {
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
      maps: Maps {
        darkness: settings.options.darkness,
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
          for monster in 0..state.monsters.len() {
            self.animate_monster(canvas, &state.players, &mut state.monsters, monster, &mut state.maps)?;
          }
          Ok(())
        })?;
        ctx.present()?;
        ctx.pump_events();
      }
    }
    Ok(true)
  }

  fn render_game_screen(&self, canvas: &mut WindowCanvas, state: &RoundState) -> Result<(), anyhow::Error> {
    canvas.copy(&self.players.texture, None, None).map_err(SdlError)?;

    self.render_level(canvas, &state.maps.level, state.maps.darkness)?;
    if state.maps.darkness {
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
          let cursor = Cursor::new(row, col);
          if DIRT_BORDER_BITMAP[level[row][col]] {
            self.render_dirt_border(canvas, cursor, level)?;
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
    cursor: Cursor,
    level: &LevelMap,
  ) -> Result<(), anyhow::Error> {
    let pos_x = (10 * cursor.col) as i32;
    let pos_y = (10 * cursor.row + 30) as i32;

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
    self.glyphs.render(
      canvas,
      pos_x,
      pos_y,
      Glyph::Monster(monster.kind, monster.facing, Digging::Hands, AnimationPhase::Phase1),
    )?;
    Ok(())
  }

  fn animate_monster(
    &self,
    canvas: &mut WindowCanvas,
    players: &[PlayerEntity],
    monsters: &mut [MonsterEntity],
    monster: usize,
    maps: &mut Maps,
  ) -> Result<(), anyhow::Error> {
    if let Some(direction) = monsters[monster].moving {
      let delta_x = monsters[monster].pos.x % 10;
      let delta_y = monsters[monster].pos.y % 10;
      let cursor = monsters[monster].pos.cursor();

      let (delta_dir, delta_orthogonal, finishing_move, can_move) = match direction {
        Direction::Left => (delta_x, delta_y, delta_x > 5, monsters[monster].pos.x > 5),
        Direction::Right => (delta_x, delta_y, delta_x < 5, monsters[monster].pos.x < 635),
        Direction::Up => (delta_y, delta_x, delta_y > 5, monsters[monster].pos.y > 35),
        Direction::Down => (delta_y, delta_x, delta_y < 5, monsters[monster].pos.x < 475),
      };

      // Vertically centered enough to be moving in the current direction
      let is_moving = can_move && delta_orthogonal > 3 && delta_orthogonal < 6;
      let map_value = maps.level[cursor.to(direction)];
      // Either finishing move into the cell or cell to the left is passable
      if is_moving && (finishing_move || map_value.is_passable()) {
        monsters[monster].pos.step(direction);
      }

      if delta_orthogonal != 5 {
        // Center our position in orthogonal direction
        monsters[monster].pos.center_orthogonal(direction);

        // Need to redraw cell orthogonal to the moving direction if we are re-centering.
        let cur = match direction {
          Direction::Left | Direction::Right if delta_orthogonal > 5 => cursor.to(Direction::Down),
          Direction::Left | Direction::Right => cursor.to(Direction::Up),
          Direction::Up | Direction::Down if delta_orthogonal > 5 => cursor.to(Direction::Right),
          Direction::Up | Direction::Down => cursor.to(Direction::Left),
        };
        self.reveal_map_square(canvas, cur, maps)?;
      }

      // We are centered in the direction we are going -- hit the map!
      if delta_dir == 5 {
        self.interact_map(canvas, players, monsters, monster, cursor.to(direction), maps)?;
      }

      // Finishing moving from adjacent square -- render that square
      if finishing_move {
        self.reveal_map_square(canvas, cursor.to(direction.reverse()), maps)?;
      }

      // Check if we need to show animation with pick axe or without
      let is_hard = delta_dir == 5
        && ((map_value >= MapValue::StoneTopLeft && map_value <= MapValue::StoneBottomRight)
          || map_value == MapValue::StoneBottomLeft
          || (map_value >= MapValue::Stone1 && map_value <= MapValue::Stone4)
          || (map_value >= MapValue::StoneLightCracked && map_value <= MapValue::StoneHeavyCracked)
          || (map_value >= MapValue::Brick && map_value <= MapValue::BrickHeavyCracked));
      let digging = if is_hard { Digging::Pickaxe } else { Digging::Hands };

      self.animate_digging(canvas, &mut monsters[monster], digging)?;
    } else {
      self.render_monster(canvas, &monsters[monster])?;
    }
    Ok(())
  }

  /// Interact with the map cell (dig it with a pickaxe, pick up gold, press buttons).
  #[allow(clippy::cognitive_complexity)]
  fn interact_map(
    &self,
    canvas: &mut WindowCanvas,
    players: &[PlayerEntity],
    monsters: &mut [MonsterEntity],
    monster: usize,
    cursor: Cursor,
    maps: &mut Maps,
  ) -> Result<(), anyhow::Error> {
    let value = maps.level[cursor];
    if value.is_passable() {
      if let Some(stats) = monsters[monster].player_stats() {
        stats.meters_ran += 1;
        if maps.darkness {
          self.reveal_view(maps)?;
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
      // Diggable squares
      // FIXME: use mapvalueset

      if maps.hits[cursor] == 30_000 {
        // 30_000 is a metal wall
      } else if maps.hits[cursor] > 1 {
        maps.hits[cursor] -= monsters[monster].drilling;
        if value.is_stone_like() {
          if maps.hits[cursor] < 500 {
            if value.is_stone_corner() {
              maps.level[cursor] = MapValue::LightGravel;
            } else {
              maps.level[cursor] = MapValue::StoneHeavyCracked;
            }
            self.reveal_map_square(canvas, cursor, maps)?;
          } else if maps.hits[cursor] < 1000 {
            if value.is_stone_corner() {
              maps.level[cursor] = MapValue::HeavyGravel;
            } else {
              maps.level[cursor] = MapValue::StoneLightCracked;
            }
            self.reveal_map_square(canvas, cursor, maps)?;
          }
        } else if value.is_brick_like() {
          if maps.hits[cursor] <= 2000 {
            maps.level[cursor] = MapValue::BrickHeavyCracked;
          } else if maps.hits[cursor] <= 4000 {
            maps.level[cursor] = MapValue::BrickLightCracked;
          }
          self.reveal_map_square(canvas, cursor, maps)?;
          return Ok(());
        }
      } else {
        maps.hits[cursor] = 0;
        maps.level[cursor] = MapValue::Passage;
        self.reveal_map_square(canvas, cursor, maps)?;
        self.render_dirt_border(canvas, cursor, &maps.level)?;
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

      if let Some(player) = monsters[monster].clone_player() {
        player.base_drillingpower += drill_value;
        player.accumulated_cash += gold_value;
      }

      monsters[monster].drilling += drill_value as i32;
      monsters[monster].accumulated_cash += gold_value;
      if value >= MapValue::SmallPickaxe && value <= MapValue::Drill {
        // FIXME: Play picaxe.voc, freq: 11000
      } else {
        // FIXME: play kili.voc, freq: 10000, 12599 or 14983
        if let Some(stats) = monsters[monster].player_stats() {
          stats.treasures_collected += 1;
        }
      }

      // FIXME: optimized re-rendering?
      self.render_players_info(canvas, players)?;

      maps.hits[cursor] = 0;
      maps.level[cursor] = MapValue::Passage;
      self.reveal_map_square(canvas, cursor, maps)?;
    } else if value == MapValue::Mine {
      // Activate the mine
      maps.timer[cursor] = 1;
    } else if PUSHABLE_BITMAP[value] {
      // We must be moving at this point, so unwrap is okay
      let target = cursor.to(monsters[monster].moving.unwrap());
      if maps.hits[cursor] == 30_000 {
        // FIXME: wall shouldn't be pushable anyways?
      } else if maps.hits[cursor] > 1 {
        // Still need to push a little
        maps.hits[cursor] -= monsters[monster].drilling;
      } else if maps.level[target].is_passable() {
        // Check if no entity is blocking the path
        if players.iter().all(|p| p.is_dead || p.pos.cursor() != target)
          && monsters.iter().all(|m| m.is_dead || m.pos.cursor() != target)
        {
          // Push to `target` locaiton
          maps.level[target] = maps.level[cursor];
          maps.timer[target] = maps.timer[cursor];
          maps.hits[target] = 24;

          // Clear old position
          maps.level[cursor] = MapValue::Passage;
          maps.timer[cursor] = 0;

          // FIXME: re-render blood
          reapply_blood(players, monsters, cursor, maps);
          self.reveal_map_square(canvas, cursor, maps)?;
          self.reveal_map_square(canvas, target, maps)?;
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
          monsters[monster].inventory[weapon] += cnt;
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
          monsters[monster].inventory[weapon] += cnt;
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
          monsters[monster].inventory[weapon] += cnt;
        }
      }

      maps.hits[cursor] = 0;
      maps.level[cursor] = MapValue::Passage;
      self.reveal_map_square(canvas, cursor, maps)?;

      // FIXME: more optimal re-rendering?
      self.render_players_info(canvas, players)?;
    } else if value == MapValue::LifeItem {
      if monsters[monster].kind == MonsterKind::Player1 {
        monsters[monster].lives += 1;
        // FIXME: we need unify players and monsters...
        self.render_players_info(canvas, players)?;
      }

      maps.hits[cursor] = 0;
      maps.level[cursor] = MapValue::Passage;
      self.reveal_map_square(canvas, cursor, maps)?;
    } else if value == MapValue::ButtonOff {
      if maps.timer[cursor] <= 1 {
        open_doors(maps);
      }
    } else if value == MapValue::ButtonOn {
      if maps.timer[cursor] <= 1 {
        close_doors(maps);
      }
    } else if value == MapValue::Teleport {
      let mut entrance_idx = 0;
      let mut teleport_count = 0;
      for row in 0..MAP_ROWS {
        for col in 0..MAP_COLS {
          if maps.level[row][col] == MapValue::Teleport {
            if cursor == Cursor::new(row, col) {
              entrance_idx = teleport_count;
            }
            teleport_count += 1;
          }
        }
      }

      let mut rng = rand::thread_rng();
      // FIXME: if teleport_count == 1
      let mut exit = rng.gen_range(0, teleport_count - 1);
      if exit >= entrance_idx {
        exit += 1;
      }

      'outer: for row in 0..MAP_ROWS {
        for col in 0..MAP_COLS {
          if maps.level[row][col] == MapValue::Teleport {
            if exit == 0 {
              // Found exit point
              self.reveal_map_square(canvas, monsters[monster].pos.cursor(), maps)?;

              // Move to the exit point
              let x = col * 10 + 5;
              let y = row * 10 + 35;
              monsters[monster].pos = Position::new(x as i32, y as i32);

              self.reveal_map_square(canvas, monsters[monster].pos.cursor(), maps)?;
              break 'outer;
            }
            exit -= 1;
          }
        }
      }
    } else if value == MapValue::Exit {
      // FIXME: exiting level
    } else if value == MapValue::Medikit {
      // FIXME: play sound picaxe.voc, freq = 11000

      // FIXME: check is_monster_active
      if true {
        monsters[monster].health = monsters[monster].kind.initial_health();
      }

      self.render_players_info(canvas, players)?;

      maps.level[cursor] = MapValue::Passage;
      self.reveal_map_square(canvas, cursor, maps)?;
    }
    Ok(())
  }

  fn reveal_view(&self, _maps: &mut Maps) -> Result<(), anyhow::Error> {
    unimplemented!("reveal view")
  }

  fn animate_digging(
    &self,
    canvas: &mut WindowCanvas,
    monster: &mut MonsterEntity,
    digging: Digging,
  ) -> Result<(), anyhow::Error> {
    let dir = if let Some(dir) = monster.moving {
      dir
    } else {
      return Ok(());
    };

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
      let glyph = Glyph::Monster(monster.kind, dir, digging, phase);
      self
        .glyphs
        .render(canvas, monster.pos.x - 5, monster.pos.y - 5, glyph)?;
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
    self
      .glyphs
      .render(canvas, (cursor.col * 10) as i32, (cursor.row * 10 + 30) as i32, glyph)?;
    maps.fog[cursor].reveal();
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

fn open_doors(_maps: &mut Maps) {
  unimplemented!()
}

fn close_doors(_maps: &mut Maps) {
  unimplemented!()
}

fn reapply_blood(players: &[PlayerEntity], monsters: &[MonsterEntity], cursor: Cursor, maps: &mut Maps) {
  for player in players {
    if player.is_dead && player.pos.cursor() == cursor {
      maps.level[cursor] = MapValue::Blood;
      return;
    }
  }
  for monster in monsters {
    if monster.is_dead && monster.pos.cursor() == cursor {
      if monster.kind == MonsterKind::Slime {
        maps.level[cursor] = MapValue::SlimeCorpse;
      } else {
        maps.level[cursor] = MapValue::Blood;
      }
      return;
    }
  }
}
