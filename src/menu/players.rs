use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::identities::Identities;
use crate::players::{PlayerStats, Players};
use crate::Application;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

//const RIGHT_PANEL_X: i32 = 376;
//const RIGHT_PANEL_Y: i32 = 22;
const LEFT_PANEL_X: i32 = 44;
const LEFT_PANEL_Y: i32 = 35;

struct State {
    total_players: u8,
    players: Players,
    identities: Identities,
    selected_player: u8,
}

impl State {
    fn active_stats(&self) -> Option<&PlayerStats> {
        if self.selected_player < 4 {
            if let Some(player) = self.identities.players[usize::from(self.selected_player)] {
                return Some(&self.players.players[player]);
            }
        }
        None
    }

    /// Move to the next menu item
    fn next_player(&mut self) {
        self.selected_player += 1;
        if self.selected_player > 4 {
            self.selected_player = 0;
        } else if self.selected_player != 4 && self.selected_player >= self.total_players {
            self.selected_player = 4;
        }
    }

    /// Move to the previous menu item
    fn previous_player(&mut self) {
        if self.selected_player == 0 {
            self.selected_player = 4;
        } else {
            self.selected_player -= 1;
            if self.selected_player >= self.total_players {
                self.selected_player = self.total_players - 1;
            }
        }
    }
}

impl Application<'_> {
    pub fn players_select_menu(
        &mut self,
        ctx: &mut ApplicationContext,
    ) -> Result<(), anyhow::Error> {
        let mut state = State {
            total_players: self.options.players,
            players: Players::load_players(ctx.game_dir())?,
            identities: Identities::load_identities(ctx.game_dir()),
            // 4 is "Play button"
            selected_player: 4,
        };
        ctx.with_render_context(|canvas| {
            canvas
                .copy(&self.players.texture, None, None)
                .map_err(SdlError)?;
            self.render_left_pane(canvas, &state)?;
            Ok(())
        })?;
        ctx.animate(Animation::FadeUp, 7)?;

        loop {
            let (scancode, _keycode) = ctx.wait_key_pressed();
            match scancode {
                Scancode::Down | Scancode::Kp2 => {
                    let previous = state.selected_player;
                    state.next_player();
                    self.render_selected_player(ctx, previous, &state)?;
                }
                Scancode::Up | Scancode::Kp8 => {
                    let previous = state.selected_player;
                    state.previous_player();
                    self.render_selected_player(ctx, previous, &state)?;
                }
                Scancode::Escape => {
                    break;
                }
                Scancode::Kp3 | Scancode::Return | Scancode::Return2 | Scancode::KpEnter
                    if state.selected_player == 4 =>
                {
                    break;
                }
                _ => {}
            }
        }
        // FIXME: save players.dat
        ctx.animate(Animation::FadeDown, 7)?;
        Ok(())
    }

    fn render_left_pane(
        &self,
        canvas: &mut WindowCanvas,
        state: &State,
    ) -> Result<(), anyhow::Error> {
        // Erase panels for unused players
        let cnt = i32::from(self.options.players);
        canvas.set_draw_color(Color::BLACK);
        for player in cnt..4 {
            let rect = Rect::new(39, player * 53 + 18, 293, 53);
            canvas.fill_rect(rect).map_err(SdlError)?;
        }

        // Original game would also render stats here, but we only render this panel when we enter
        // the menu, so none of the players is selected.
        let y = i32::from(state.selected_player) * 53 + LEFT_PANEL_Y;
        self.glyphs
            .render(canvas, LEFT_PANEL_X, y, Glyph::ShovelPointer)?;

        self.render_left_pane_names(canvas, state)?;
        Ok(())
    }

    fn render_left_pane_names(
        &self,
        canvas: &mut WindowCanvas,
        state: &State,
    ) -> Result<(), anyhow::Error> {
        for player in 0..4i32 {
            canvas.set_draw_color(Color::BLACK);
            // Maximum name length is 26
            canvas
                .fill_rect(Rect::new(119, player * 53 + 40, 26 * 8, 10))
                .map_err(SdlError)?;
            if player < i32::from(self.options.players) {
                let color = self.players.palette[1];
                if let Some(idx) = state.identities.players[player as usize] {
                    let name = &state.players.players[idx].name;
                    self.font
                        .render(canvas, 120, player * 53 + 41, color, name)?;
                }
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
            let old_y = i32::from(previous) * 53 + LEFT_PANEL_Y;
            let (w, h) = Glyph::ShovelPointer.dimensions();
            canvas.set_draw_color(Color::BLACK);
            canvas
                .fill_rect(Rect::new(LEFT_PANEL_X, old_y, w, h))
                .map_err(SdlError)?;

            let y = i32::from(state.selected_player) * 53 + LEFT_PANEL_Y;
            self.glyphs
                .render(canvas, LEFT_PANEL_X, y, Glyph::ShovelPointer)?;

            self.render_stats(canvas, state.active_stats())?;
            Ok(())
        })?;
        ctx.present()?;
        Ok(())
    }

    fn render_stats(
        &self,
        canvas: &mut WindowCanvas,
        stats: Option<&PlayerStats>,
    ) -> Result<(), anyhow::Error> {
        canvas.set_draw_color(Color::BLACK);

        // Individual stats indicators
        for row in 0..6 {
            for column in 0..2 {
                let x = column * 146 + 64;
                let y = row * 24 + 328;
                canvas
                    .fill_rect(Rect::new(x, y, 95, 10))
                    .map_err(SdlError)?;
            }
        }

        // Player history
        canvas
            .fill_rect(Rect::new(367, 328, 198, 130))
            .map_err(SdlError)?;

        let _stats = if let Some(stats) = stats {
            stats
        } else {
            return Ok(());
        };

        // FIXME: render stats

        Ok(())
    }
}
