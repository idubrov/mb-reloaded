//! Player selection menu.
//!
//! Note that this screen in particular behaves a bit differently from the original one.
use crate::context::{Animation, ApplicationContext, InputEvent};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::identities::Identities;
use crate::roster::{PlayersRoster, RosterInfo};
use crate::Application;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

const RIGHT_PANEL_X: i32 = 376;
const RIGHT_PANEL_Y: i32 = 22;
const LEFT_PANEL_X: i32 = 44;
const LEFT_PANEL_Y: i32 = 35;

struct State {
  players: u8,
  roster: PlayersRoster,
  identities: Identities,
  active_player: u8,
}

impl State {
  /// Return stats for the player with the given index
  fn stats(&self, idx: u8) -> Option<&RosterInfo> {
    self.roster.players[usize::from(idx)].as_ref()
  }

  fn active_stats(&self) -> Option<&RosterInfo> {
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
    } else if self.active_player != 4 && self.active_player >= self.players {
      self.active_player = 4;
    }
  }

  /// Move to the previous menu item
  fn previous_player(&mut self) {
    if self.active_player == 0 {
      self.active_player = 4;
    } else {
      self.active_player -= 1;
      if self.active_player >= self.players {
        self.active_player = self.players - 1;
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
    self.roster.players[usize::from(idx)] = None;
    for identity in &mut self.identities.players {
      if *identity == Some(idx) {
        *identity = None;
      }
    }
  }

  /// `true` if all players were selected
  fn all_selected(&self) -> bool {
    self
      .identities
      .players
      .iter()
      .take(usize::from(self.players))
      .all(Option::is_some)
  }
}

impl Application<'_> {
  /// Returns selected players. If F10 was pressed (exit), returns an empty list.
  pub fn players_select_menu(
    &self,
    ctx: &mut ApplicationContext,
    total_players: u8,
  ) -> Result<Vec<RosterInfo>, anyhow::Error> {
    let mut state = State {
      players: total_players,
      roster: PlayersRoster::load(ctx.game_dir())?,
      identities: Identities::load(ctx.game_dir()),
      // 4 is "Play button"
      active_player: 4,
    };
    ctx.with_render_context(|canvas| {
      canvas
        .copy(&self.select_players.texture, None, None)
        .map_err(SdlError)?;
      self.render_left_pane(canvas, &state, state.active_player)?;
      self.render_right_pane(canvas, &state)?;
      Ok(())
    })?;
    ctx.animate(Animation::FadeUp, 7)?;

    let exit = loop {
      let last_active_player = state.active_player;

      match ctx.wait_input_event() {
        InputEvent::TextInput(text) if state.active_player != 4 => {
          // Special case: when typing text over a player, immediately create a new player in the
          // roster and start editing it.
          let selection = self.players_name_select_menu(ctx, &mut state, Some(text))?;
          state.select_player(selection);
        }
        InputEvent::TextInput(_) => continue,
        InputEvent::KeyPress(scancode, _keycode) => match scancode {
          Scancode::Down | Scancode::Kp2 => state.next_player(),
          Scancode::Up | Scancode::Kp8 => state.previous_player(),
          Scancode::Escape => {
            // Check that all players were selected
            if state.all_selected() {
              break false;
            }
          }
          Scancode::Kp6 | Scancode::Return | Scancode::Return2 | Scancode::KpEnter | Scancode::Right
            if state.active_player == 4 =>
          {
            if state.all_selected() {
              break false;
            }
          }
          Scancode::F10 => {
            break true;
          }
          Scancode::Kp6 | Scancode::Return | Scancode::Return2 | Scancode::KpEnter | Scancode::Right => {
            let selection = self.players_name_select_menu(ctx, &mut state, None)?;
            state.select_player(selection);
          }

          _ => {
            // Skip re-rendering nothing changed.
            continue;
          }
        },
      };

      ctx.with_render_context(|canvas| {
        self.render_left_pane(canvas, &state, last_active_player)?;
        Ok(())
      })?;
      ctx.present()?;
    };

    state.identities.save(ctx.game_dir())?;
    state.roster.save(ctx.game_dir())?;
    ctx.animate(Animation::FadeDown, 7)?;

    let mut selected = Vec::new();
    if !exit {
      selected.reserve(usize::from(total_players));
      for idx in 0..total_players {
        let roster_index = state.identities.players[usize::from(idx)].unwrap();
        selected.push(
          state.roster.players[usize::from(roster_index)]
            .as_ref()
            .unwrap()
            .clone(),
        );
      }
    }
    Ok(selected)
  }

  fn players_name_select_menu(
    &self,
    ctx: &mut ApplicationContext,
    state: &mut State,
    mut initial_input: Option<String>,
  ) -> Result<Option<u8>, anyhow::Error> {
    let current_player = usize::from(state.active_player);

    // If we entered this menu via pressed key, pick an empty name slot
    if initial_input.is_some() {
      let player_idx = state.roster.players.iter().position(|v| v.is_none());
      state.identities.players[current_player] = Some(player_idx.unwrap_or(31) as u8);
    }

    let mut arrow_pos = state.identities.players[current_player].unwrap_or(0);
    ctx.with_render_context(|canvas| self.render_arrow_pointer(canvas, arrow_pos))?;
    ctx.present()?;

    let selection = loop {
      let scancode = match initial_input
        .take()
        .map(InputEvent::TextInput)
        .unwrap_or_else(|| ctx.wait_input_event())
      {
        InputEvent::KeyPress(scancode, _keycode) => scancode,
        InputEvent::TextInput(text) => {
          self.edit_new_player_name(ctx, state, arrow_pos, Some(text))?;
          continue;
        }
      };

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
          if state.stats(arrow_pos).is_some() {
            break Some(arrow_pos);
          } else {
            break None;
          }
        }
        // No selection
        // FIXME: on F10, should exit from player selection screen
        Scancode::Escape | Scancode::F10 => break None,
        // Delete currently selected player
        Scancode::Backspace | Scancode::Delete => {
          state.delete_stats(arrow_pos);
          ctx.with_render_context(|canvas| self.render_right_pane(canvas, state))?;
          ctx.present()?;
        }

        Scancode::Return | Scancode::KpEnter | Scancode::Return2 => {
          self.edit_new_player_name(ctx, state, arrow_pos, None)?;
        }

        _ => {}
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

  /// Menu loop for editing characters in a new player name.
  fn edit_new_player_name(
    &self,
    ctx: &mut ApplicationContext,
    state: &mut State,
    player_idx: u8,
    mut first: Option<String>,
  ) -> Result<(), anyhow::Error> {
    let x = RIGHT_PANEL_X + 2;
    let y = RIGHT_PANEL_Y + (player_idx as i32) * 8 + 1;

    // Initial edit line
    ctx.with_render_context(|canvas| {
      canvas.set_draw_color(Color::BLACK);
      let rect = Rect::new(x, y, 192, 8);
      canvas.fill_rect(rect).map_err(SdlError)?;
      canvas.set_draw_color(self.select_players.palette[8]);
      let rect = Rect::new(x + 1, y + 6, 8, 2);
      canvas.fill_rect(rect).map_err(SdlError)?;
      Ok(())
    })?;

    let mut name = String::new();
    loop {
      match first
        .take()
        .map(InputEvent::TextInput)
        .unwrap_or_else(|| ctx.wait_input_event())
      {
        InputEvent::KeyPress(scancode, _) => match scancode {
          Scancode::Return | Scancode::Return2 | Scancode::KpEnter | Scancode::Escape => {
            // We are done -- exit the loop
            break;
          }
          Scancode::Delete | Scancode::Backspace => {
            if !name.is_empty() {
              name.truncate(name.len() - 1);
            }
          }
          _ => continue,
        },
        InputEvent::TextInput(text) => {
          for ch in text.chars() {
            if ch.is_ascii() {
              name.push(ch);
            }
          }
          if name.len() > 24 {
            name.truncate(24);
          }
        }
      }

      // Re-render the name and the cursor
      ctx.with_render_context(|canvas| {
        canvas.set_draw_color(Color::BLACK);
        let rect = Rect::new(x, y, 193, 8);
        canvas.fill_rect(rect).map_err(SdlError)?;
        self.font.render(canvas, x, y, self.select_players.palette[1], &name)?;

        if name.len() < 24 {
          canvas.set_draw_color(self.select_players.palette[8]);
          let rect = Rect::new(x + 1 + 8 * (name.len() as i32), y + 6, 8, 2);
          canvas.fill_rect(rect).map_err(SdlError)?;
        }

        Ok(())
      })?;
      ctx.present()?;
    }

    let mut new_player = RosterInfo::default();
    new_player.name = name;
    state.roster.players[usize::from(player_idx)] = Some(new_player);

    // Refresh names panel
    ctx.with_render_context(|canvas| self.render_right_pane(canvas, state))?;
    ctx.present()?;
    Ok(())
  }

  fn render_left_pane(
    &self,
    canvas: &mut WindowCanvas,
    state: &State,
    last_active_player: u8,
  ) -> Result<(), anyhow::Error> {
    // Erase panels for unused players
    let cnt = i32::from(state.players);
    canvas.set_draw_color(Color::BLACK);
    for player in cnt..4 {
      let rect = Rect::new(39, player * 53 + 18, 293, 53);
      canvas.fill_rect(rect).map_err(SdlError)?;
    }

    self.render_shovel_pointer(canvas, last_active_player, state.active_player)?;
    self.render_left_pane_names(canvas, state)?;
    self.render_stats(canvas, state.active_stats())?;
    Ok(())
  }

  fn render_left_pane_names(&self, canvas: &mut WindowCanvas, state: &State) -> Result<(), anyhow::Error> {
    for player in 0..4i32 {
      canvas.set_draw_color(Color::BLACK);
      // Maximum name length is 26
      canvas
        .fill_rect(Rect::new(119, player * 53 + 40, 26 * 8, 10))
        .map_err(SdlError)?;
      if player < i32::from(state.players) {
        let color = self.select_players.palette[1];
        if let Some(stats) =
          state.identities.players[player as usize].and_then(|idx| state.roster.players[usize::from(idx)].as_ref())
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

    let palette = &self.select_players.palette;
    for idx in 0..32 {
      let x = RIGHT_PANEL_X + 2;
      let y = RIGHT_PANEL_Y + (idx as i32) * 8 + 1;
      if let Some(ref player) = state.roster.players[idx] {
        self.font.render(canvas, x, y, palette[1], &player.name)?;
      } else {
        self.font.render(canvas, x, y, palette[3], "-")?;
      }
    }

    Ok(())
  }

  /// Update rendering of the shovel pointer
  fn render_shovel_pointer(&self, canvas: &mut WindowCanvas, previous: u8, current: u8) -> Result<(), anyhow::Error> {
    if previous != current {
      // Erase old pointer
      let old_y = i32::from(previous) * 53 + LEFT_PANEL_Y;
      let (w, h) = Glyph::ShovelPointer.dimensions();
      canvas.set_draw_color(Color::BLACK);
      canvas
        .fill_rect(Rect::new(LEFT_PANEL_X, old_y, w, h))
        .map_err(SdlError)?;
    }

    // Render the new pointer
    let y = i32::from(current) * 53 + LEFT_PANEL_Y;
    self.glyphs.render(canvas, LEFT_PANEL_X, y, Glyph::ShovelPointer)?;
    Ok(())
  }

  /// Render player statistics
  fn render_stats(&self, canvas: &mut WindowCanvas, stats: Option<&RosterInfo>) -> Result<(), anyhow::Error> {
    let white = self.select_players.palette[1];
    let red_color = self.select_players.palette[3];

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
    let palette = &self.select_players.palette;
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
