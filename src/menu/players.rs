//! Player selection menu.
//!
//! Note that this screen in particular behaves a bit differently from the original one.
use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::identities::Identities;
use crate::players::{PlayerStats, Players};
use crate::Application;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

const RIGHT_PANEL_X: i32 = 376;
const RIGHT_PANEL_Y: i32 = 22;
const LEFT_PANEL_X: i32 = 44;
const LEFT_PANEL_Y: i32 = 35;

struct State {
  total_players: u8,
  players: Players,
  identities: Identities,
  active_player: u8,
}

impl State {
  /// Return stats for the player with the given index
  fn stats(&self, idx: u8) -> Option<&PlayerStats> {
    self.players.players[usize::from(idx)].as_ref()
  }

  fn active_stats(&self) -> Option<&PlayerStats> {
    if self.active_player < 4 {
      if let Some(player) = self.identities.players[usize::from(self.active_player)] {
        return self.stats(player);
      }
    }
    None
  }

  /// Move to the next menu item
  fn next_player(&mut self) {
    self.active_player += 1;
    if self.active_player > 4 {
      self.active_player = 0;
    } else if self.active_player != 4 && self.active_player >= self.total_players {
      self.active_player = 4;
    }
  }

  /// Move to the previous menu item
  fn previous_player(&mut self) {
    if self.active_player == 0 {
      self.active_player = 4;
    } else {
      self.active_player -= 1;
      if self.active_player >= self.total_players {
        self.active_player = self.total_players - 1;
      }
    }
  }

  /// Make currently active player slot to use given player identity (if it's not `None`).
  fn select_player(&mut self, selection: Option<u8>) {
    if selection.is_some() {
      self.identities.players[usize::from(self.active_player)] = selection;
    }
  }

  /// Delete statistics for the given player index
  fn delete_stats(&mut self, idx: u8) {
    self.players.players[usize::from(idx)] = None;
    for identity in &mut self.identities.players {
      if *identity == Some(idx) {
        *identity = None;
      }
    }
  }
}

impl Application<'_> {
  pub fn players_select_menu(&mut self, ctx: &mut ApplicationContext) -> Result<(), anyhow::Error> {
    let mut state = State {
      total_players: self.options.players,
      players: Players::load_players(ctx.game_dir())?,
      identities: Identities::load_identities(ctx.game_dir()),
      // 4 is "Play button"
      active_player: 4,
    };
    ctx.with_render_context(|canvas| {
      canvas.copy(&self.players.texture, None, None).map_err(SdlError)?;
      self.render_left_pane(canvas, &state)?;
      self.render_right_pane(canvas, &state)?;
      Ok(())
    })?;
    ctx.animate(Animation::FadeUp, 7)?;

    loop {
      let (scancode, _keycode) = ctx.wait_key_pressed();
      match scancode {
        Scancode::Down | Scancode::Kp2 => {
          let previous = state.active_player;
          state.next_player();
          self.render_selected_player(ctx, previous, &state)?;
        }
        Scancode::Up | Scancode::Kp8 => {
          let previous = state.active_player;
          state.previous_player();
          self.render_selected_player(ctx, previous, &state)?;
        }
        Scancode::Escape => {
          break;
        }
        Scancode::Kp6 | Scancode::Return | Scancode::Return2 | Scancode::KpEnter | Scancode::Right
          if state.active_player == 4 =>
        {
          break;
        }
        Scancode::Kp6 | Scancode::Return | Scancode::Return2 | Scancode::KpEnter | Scancode::Right => {
          let selection = self.players_name_select_menu(ctx, &mut state, None)?;
          state.select_player(selection);

          ctx.with_render_context(|canvas| {
            self.render_left_pane(canvas, &state)?;
            self.render_stats(canvas, state.active_stats())?;
            Ok(())
          })?;
          ctx.present()?;
        }

        // FIXME: Escape is start the game if all players are selected
        // FIXME: F10 is exit the game
        _ => {}
      }
    }
    // FIXME: save players.dat
    ctx.animate(Animation::FadeDown, 7)?;
    Ok(())
  }

  fn players_name_select_menu(
    &self,
    ctx: &mut ApplicationContext,
    state: &mut State,
    mut initial_keycode: Option<Keycode>,
  ) -> Result<Option<u8>, anyhow::Error> {
    let current_player = usize::from(state.active_player);

    // If we entered this menu via pressed key, pick empty name slot (or the last one, if no empty slots)
    if initial_keycode.is_some() && state.identities.players[current_player].is_none() {
      let player_idx = state.players.players.iter().position(|v| v.is_none());
      state.identities.players[current_player] = Some(player_idx.unwrap_or(31) as u8);
    }

    let mut arrow_pos = state.identities.players[current_player].unwrap_or(0);
    ctx.with_render_context(|canvas| self.render_arrow_pointer(canvas, arrow_pos))?;
    ctx.present()?;

    let selection = loop {
      let (scancode, keycode) = initial_keycode
        .take()
        .map(|keycode| (Scancode::Application, keycode))
        .unwrap_or_else(|| ctx.wait_key_pressed());
      let last_arrow_pos = arrow_pos;
      match scancode {
        Scancode::Down | Scancode::Kp2 => {
          arrow_pos = (arrow_pos + 1) % 32;
        }
        Scancode::Up | Scancode::Kp8 => {
          arrow_pos = (arrow_pos + 31) % 32;
        }
        Scancode::Left | Scancode::Kp4 => {
          // If we have player for the current index configured, pick it
          if state.players.players[usize::from(arrow_pos)].is_some() {
            break Some(arrow_pos);
          } else {
            break None;
          }
        }
        // No selection
        Scancode::Escape => break None,
        // Delete currently selected player
        Scancode::Backspace | Scancode::Delete => {
          state.delete_stats(arrow_pos);

          ctx.with_render_context(|canvas| self.render_right_pane(canvas, state))?;
          ctx.present()?;
        }

        _other => {
          self.players_name_enter(ctx)?;

          // Re-render name.
        }
      }

      if last_arrow_pos != arrow_pos {
        ctx.with_render_context(|canvas| {
          self.clear_arrow_pointer(canvas, last_arrow_pos)?;
          self.render_arrow_pointer(canvas, arrow_pos)?;
          self.render_stats(canvas, state.stats(arrow_pos))?;
          Ok(())
        })?;
        ctx.present()?;
      }
    };

    ctx.with_render_context(|canvas| self.clear_arrow_pointer(canvas, arrow_pos))?;
    ctx.present()?;

    Ok(selection)
  }

  fn players_name_enter(&self, ctx: &mut ApplicationContext) -> Result<(), anyhow::Error> {
    Ok(())
  }

  fn render_left_pane(&self, canvas: &mut WindowCanvas, state: &State) -> Result<(), anyhow::Error> {
    // Erase panels for unused players
    let cnt = i32::from(self.options.players);
    canvas.set_draw_color(Color::BLACK);
    for player in cnt..4 {
      let rect = Rect::new(39, player * 53 + 18, 293, 53);
      canvas.fill_rect(rect).map_err(SdlError)?;
    }

    // Original game would also render stats here, but we only render this panel when we enter
    // the menu, so none of the players is selected.
    self.render_shovel_pointer(canvas, state.active_player, state.active_player)?;
    self.render_left_pane_names(canvas, state)?;
    Ok(())
  }

  fn render_left_pane_names(&self, canvas: &mut WindowCanvas, state: &State) -> Result<(), anyhow::Error> {
    for player in 0..4i32 {
      canvas.set_draw_color(Color::BLACK);
      // Maximum name length is 26
      canvas
        .fill_rect(Rect::new(119, player * 53 + 40, 26 * 8, 10))
        .map_err(SdlError)?;
      if player < i32::from(self.options.players) {
        let color = self.players.palette[1];
        if let Some(stats) =
          state.identities.players[player as usize].and_then(|idx| state.players.players[usize::from(idx)].as_ref())
        {
          self.font.render(canvas, 120, player * 53 + 41, color, &stats.name)?;
        }
      }
    }
    Ok(())
  }

  fn render_right_pane(&self, canvas: &mut WindowCanvas, state: &State) -> Result<(), anyhow::Error> {
    canvas.set_draw_color(Color::BLACK);
    let rect = Rect::new(RIGHT_PANEL_X + 2, RIGHT_PANEL_Y + 1, 198, 256);
    canvas.fill_rect(rect).map_err(SdlError)?;

    let palette = &self.players.palette;
    for idx in 0..32 {
      let x = RIGHT_PANEL_X + 2;
      let y = RIGHT_PANEL_Y + (idx as i32) * 8 + 1;
      if let Some(ref player) = state.players.players[idx] {
        self.font.render(canvas, x, y, palette[1], &player.name)?;
      } else {
        self.font.render(canvas, x, y, palette[3], "-")?;
      }
    }

    Ok(())
  }

  /// Render update to a selected player in the left panel and also in the stats
  fn render_selected_player(
    &self,
    ctx: &mut ApplicationContext,
    previous: u8,
    state: &State,
  ) -> Result<(), anyhow::Error> {
    ctx.with_render_context(|canvas| {
      self.render_shovel_pointer(canvas, previous, state.active_player)?;
      self.render_stats(canvas, state.active_stats())?;
      Ok(())
    })?;
    ctx.present()?;
    Ok(())
  }

  /// Update rendering of the shovel pointer
  fn render_shovel_pointer(&self, canvas: &mut WindowCanvas, previous: u8, current: u8) -> Result<(), anyhow::Error> {
    // Erase old pointer
    let old_y = i32::from(previous) * 53 + LEFT_PANEL_Y;
    let (w, h) = Glyph::ShovelPointer.dimensions();
    canvas.set_draw_color(Color::BLACK);
    canvas
      .fill_rect(Rect::new(LEFT_PANEL_X, old_y, w, h))
      .map_err(SdlError)?;

    // Render the new pointer
    let y = i32::from(current) * 53 + LEFT_PANEL_Y;
    self.glyphs.render(canvas, LEFT_PANEL_X, y, Glyph::ShovelPointer)?;
    Ok(())
  }

  /// Render player statistics
  fn render_stats(&self, canvas: &mut WindowCanvas, stats: Option<&PlayerStats>) -> Result<(), anyhow::Error> {
    let white = self.players.palette[1];
    let red_color = self.players.palette[3];

    canvas.set_draw_color(Color::BLACK);

    // Individual stats indicators
    for row in 0..6 {
      for column in 0..2 {
        let x = column * 146 + 64;
        let y = row * 24 + 328;
        canvas.fill_rect(Rect::new(x, y, 95, 10)).map_err(SdlError)?;
      }
    }

    // Player past history
    canvas.fill_rect(Rect::new(367, 328, 198, 130)).map_err(SdlError)?;

    let stats = if let Some(stats) = stats {
      stats
    } else {
      return Ok(());
    };

    // Tournaments and rounds
    for (idx, (total, wins)) in [
      (stats.tournaments, stats.tournaments_wins),
      (stats.rounds, stats.rounds_wins),
    ]
    .iter()
    .copied()
    .enumerate()
    {
      let idx = idx as i32;
      self
        .font
        .render(canvas, 65, 330 + 72 * idx, white, &total.to_string())?;
      self.font.render(canvas, 65, 354 + 72 * idx, white, &wins.to_string())?;
      if total != 0 {
        let width = 1 + (94 * wins) / total;
        canvas.set_draw_color(white);
        canvas
          .fill_rect(Rect::new(64, 376 + 72 * idx, width, 10))
          .map_err(SdlError)?;

        let percentage = (200 * wins + total) / total / 2;
        self
          .font
          .render(canvas, 65, 378 + 72 * idx, red_color, &format!("{}%", percentage))?;
      }
    }

    self
      .font
      .render(canvas, 211, 330, white, &stats.treasures_collected.to_string())?;
    self
      .font
      .render(canvas, 211, 354, white, &stats.total_money.to_string())?;
    self
      .font
      .render(canvas, 211, 378, white, &stats.bombs_bought.to_string())?;
    self
      .font
      .render(canvas, 211, 402, white, &stats.bombs_dropped.to_string())?;
    self.font.render(canvas, 211, 426, white, &stats.deaths.to_string())?;
    self
      .font
      .render(canvas, 211, 450, white, &stats.meters_ran.to_string())?;

    let mut offset = (stats.tournaments as usize) % 34;
    let mut last_x = 367;
    let mut last_y = 457 - i32::from(stats.history[offset]);
    let palette = &self.players.palette;
    for _ in 1..34 {
      offset = (offset + 1) % 34;
      let value = stats.history[offset];
      let y = 457 - i32::from(value);
      let color = match (u16::from(value) * 4 + 67) / 134 {
        0 => palette[3],
        1 => palette[7],
        2 => palette[6],
        3 => palette[5],
        _ => palette[4],
      };
      canvas.set_draw_color(color);
      canvas.draw_line((last_x, last_y), (last_x + 5, y)).map_err(SdlError)?;
      canvas.draw_line((last_x + 5, y), (last_x + 6, y)).map_err(SdlError)?;

      last_x += 6;
      last_y = y;
    }
    Ok(())
  }

  /// Update rendering of the arrow pointer in the right panel
  fn clear_arrow_pointer(&self, canvas: &mut WindowCanvas, position: u8) -> Result<(), anyhow::Error> {
    let old_y = i32::from(position) * 8 + RIGHT_PANEL_Y;
    let (w, h) = Glyph::ArrowPointer.dimensions();
    canvas.set_draw_color(Color::BLACK);
    canvas
      .fill_rect(Rect::new(RIGHT_PANEL_X - 37, old_y, w, h))
      .map_err(SdlError)?;
    Ok(())
  }

  /// Update rendering of the arrow pointer in the right panel
  fn render_arrow_pointer(&self, canvas: &mut WindowCanvas, position: u8) -> Result<(), anyhow::Error> {
    let y = i32::from(position) * 8 + RIGHT_PANEL_Y;
    self.glyphs.render(canvas, RIGHT_PANEL_X - 37, y, Glyph::ArrowPointer)?;
    Ok(())
  }
}
