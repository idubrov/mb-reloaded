use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::Application;
use rand::prelude::*;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::path::{Path, PathBuf};

// paletted indices
const SELECTED: usize = 7;
const UNSELECTED: usize = 1;
const SELECTED_RANDOM: usize = 5;
const UNSELECTED_RANDOM: usize = 4;
const ACTIVE_SELECTED: usize = 6;
const ACTIVE_UNSELECTED: usize = 0;

struct State {
    levels: Vec<PathBuf>,
    cursor: usize,
    /// Length is `levels.len() + 1` as we also track "Random"
    selected: Vec<bool>,
    level_pick: Vec<Option<PathBuf>>,
}

impl State {
    fn select_current(&mut self) {
        if self.cursor == 0 {
            self.level_pick.push(None);
        } else {
            self.level_pick
                .push(Some(self.levels[self.cursor - 1].to_owned()));
        }
        self.selected[self.cursor] = true;
    }

    fn left(&mut self) {
        if (self.cursor % 8) != 0 {
            self.cursor -= 1;
        }
    }

    fn up(&mut self) {
        if self.cursor >= 8 {
            self.cursor -= 8;
        }
    }

    fn right(&mut self) {
        if (self.cursor % 8) != 7 && self.cursor < self.levels.len() {
            self.cursor += 1;
        }
    }

    fn down(&mut self) {
        if (self.cursor / 8) < 41 && self.cursor + 8 <= self.levels.len() {
            self.cursor += 8;
        }
    }

    fn randomize(&mut self, count: usize) {
        self.level_pick.clear();
        self.selected.truncate(0);
        self.selected.resize(self.levels.len() + 1, false);

        let mut indices: Vec<usize> = (0..self.levels.len()).collect();
        while self.level_pick.len() < usize::from(count) {
            let mut rng = rand::thread_rng();
            indices.shuffle(&mut rng);
            let remaining = usize::from(count) - self.level_pick.len();
            for index in &indices[..remaining.min(indices.len())] {
                self.selected[index + 1] = true;
                self.level_pick.push(Some(self.levels[*index].to_owned()));
            }
        }
    }
}

impl Application {
    pub fn load_levels(&mut self, ctx: &mut ApplicationContext) -> Result<(), anyhow::Error> {
        let mut levels = find_levels(ctx.game_dir())?;

        // We cannot show more than that
        levels.truncate(327);
        if levels.is_empty() {
            ctx.with_render_context(|canvas| {
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                let color = self.main_menu.palette[1];
                self.font.render(
                    canvas,
                    130,
                    236,
                    color,
                    "No maps to load!!! (You can create maps with MINEDIT!!!)",
                )?;
                self.font.render(
                    canvas,
                    130,
                    248,
                    color,
                    "    Press any key to return to the options menu",
                )?;
                Ok(())
            })?;
            ctx.animate(Animation::FadeUp, 7)?;
            ctx.wait_key_pressed();
            ctx.animate(Animation::FadeDown, 7)?;
            return Ok(());
        }

        let mut selected = Vec::new();
        selected.resize(levels.len() + 1, false);
        let state = State {
            levels,
            cursor: 0,
            selected,
            level_pick: Vec::new(),
        };

        self.render_levels_menu(ctx, &state)?;
        ctx.animate(Animation::FadeUp, 7)?;
        let _selected = self.level_select_loop(ctx, state)?;
        ctx.animate(Animation::FadeDown, 7)?;
        Ok(())
    }

    fn level_select_loop(
        &self,
        ctx: &mut ApplicationContext,
        mut state: State,
    ) -> Result<Vec<Option<PathBuf>>, anyhow::Error> {
        loop {
            let (scan, _) = ctx.wait_key_pressed();
            let last_cursor = state.cursor;
            let mut need_update = false;
            match scan {
                Scancode::Escape => break,
                Scancode::Return | Scancode::KpEnter
                    if state.level_pick.len() < usize::from(self.options.rounds) =>
                {
                    state.select_current();
                    need_update = true;
                }
                Scancode::Left => state.left(),
                Scancode::Up => state.up(),
                Scancode::Right => state.right(),
                Scancode::Down => state.down(),

                Scancode::F1 => {
                    state.randomize(usize::from(self.options.rounds));

                    // Refresh the whole menu
                    ctx.with_render_context(|canvas| {
                        for idx in 0..=state.levels.len() {
                            self.render_slot(canvas, &state, idx)?;
                        }
                        self.render_selected_count(canvas, state.level_pick.len())?;
                        Ok(())
                    })?;
                    ctx.present()?;
                }
                _ => {}
            }

            if last_cursor != state.cursor || need_update {
                ctx.with_render_context(|canvas| {
                    self.render_selected_count(canvas, state.level_pick.len())?;
                    self.render_slot(canvas, &state, last_cursor)?;
                    self.render_slot(canvas, &state, state.cursor)?;
                    Ok(())
                })?;
                ctx.present()?;
            }
        }
        Ok(state.level_pick)
    }

    fn render_slot(
        &self,
        canvas: &mut WindowCanvas,
        state: &State,
        position: usize,
    ) -> Result<(), anyhow::Error> {
        let selected = state.selected[position];
        let active = state.cursor == position;
        let path = if position == 0 {
            None
        } else {
            Some(&state.levels[position - 1])
        };

        let column = (position % 8) as i32;
        let row = (position / 8) as i32;
        let rect = Rect::new(column * 80, row * 10 + 74, 70, 8);

        if active {
            canvas.set_draw_color(self.levels_menu.palette[1]);
        } else {
            canvas.set_draw_color(self.levels_menu.palette[0]);
        }
        canvas.fill_rect(rect).map_err(SdlError)?;

        let color = match (selected, active) {
            (false, _) if position == 0 => UNSELECTED_RANDOM,
            (true, _) if position == 0 => SELECTED_RANDOM,
            (false, false) => UNSELECTED,
            (true, false) => SELECTED,
            (false, true) => ACTIVE_UNSELECTED,
            (true, true) => ACTIVE_SELECTED,
        };
        let slot;
        let filename = if let Some(path) = path {
            slot = path.file_stem().unwrap().to_string_lossy().to_uppercase();
            &slot
        } else {
            "Random"
        };

        let left = (column * 80) as i32;
        let top = (row * 10 + 74) as i32;
        self.font
            .render(canvas, left, top, self.levels_menu.palette[color], filename)?;
        Ok(())
    }

    fn render_selected_count(
        &self,
        canvas: &mut WindowCanvas,
        selected: usize,
    ) -> Result<(), anyhow::Error> {
        canvas.set_draw_color(Color::BLACK);
        canvas
            .fill_rect(Rect::new(15, 15, 24, 8))
            .map_err(SdlError)?;
        self.font.render(
            canvas,
            15,
            15,
            self.levels_menu.palette[1],
            &selected.to_string(),
        )?;
        Ok(())
    }

    fn render_levels_menu(
        &self,
        ctx: &mut ApplicationContext,
        state: &State,
    ) -> Result<(), anyhow::Error> {
        ctx.with_render_context(|canvas| {
            canvas
                .copy(&self.levels_menu.texture, None, None)
                .map_err(SdlError)?;
            for idx in 0..=state.levels.len() {
                self.render_slot(canvas, state, idx)?;
            }
            self.render_selected_count(canvas, state.level_pick.len())?;
            Ok(())
        })?;
        Ok(())
    }
}

fn find_levels(path: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
    let mut result = Vec::new();
    for entry in path.read_dir()? {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |f| f == "mne" || f == "MNE") {
                result.push(path.to_owned());
            }
        }
    }
    result.sort();
    Ok(result)
}
