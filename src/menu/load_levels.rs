use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::map::MapData;
use crate::Application;
use rand::prelude::*;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::collections::hash_map::{Entry, HashMap};
use std::path::Path;

// paletted indices
const SELECTED: usize = 7;
const UNSELECTED: usize = 1;
const SELECTED_RANDOM: usize = 5;
const UNSELECTED_RANDOM: usize = 4;
const ACTIVE_SELECTED: usize = 6;
const ACTIVE_UNSELECTED: usize = 0;

enum LevelInfo {
    Random,
    File { name: String, map: MapData },
}

struct State {
    levels: Vec<LevelInfo>,
    cursor: usize,
    level_pick: Vec<usize>,
}

impl State {
    fn select_current(&mut self) {
        self.level_pick.push(self.cursor);
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

        // Don't pick random
        let mut indices: Vec<usize> = (1..self.levels.len()).collect();
        while self.level_pick.len() < count {
            let mut rng = rand::thread_rng();
            indices.shuffle(&mut rng);
            let remaining = count - self.level_pick.len();
            for index in &indices[..remaining.min(indices.len())] {
                self.level_pick.push(*index);
            }
        }
    }
}

impl Application<'_> {
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

        let state = State {
            levels,
            cursor: 0,
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
    ) -> Result<Vec<usize>, anyhow::Error> {
        let mut previews = HashMap::new();
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
                        for idx in 0..state.levels.len() {
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
                let texture_creator = ctx.texture_creator();
                ctx.with_render_context(|canvas| {
                    self.render_selected_count(canvas, state.level_pick.len())?;
                    self.render_slot(canvas, &state, last_cursor)?;
                    self.render_slot(canvas, &state, state.cursor)?;
                    if last_cursor != state.cursor {
                        let preview = match previews.entry(state.cursor) {
                            Entry::Occupied(v) => v.into_mut(),
                            Entry::Vacant(v) => {
                                let texture = self.generate_preview(
                                    texture_creator,
                                    &state.levels[state.cursor],
                                )?;
                                v.insert(texture)
                            }
                        };
                        if let Some(preview) = preview {
                            let rect = Rect::new(330, 7, 64, 45);
                            canvas.copy(preview, None, rect).map_err(SdlError)?;
                        }
                    }
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
        let selected = state.level_pick.contains(&position);
        let active = state.cursor == position;
        let level = &state.levels[position];

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
        let left = (column * 80) as i32;
        let top = (row * 10 + 74) as i32;
        let level_name = match level {
            LevelInfo::Random => "Random",
            LevelInfo::File { ref name, .. } => name,
        };
        self.font.render(
            canvas,
            left,
            top,
            self.levels_menu.palette[color],
            level_name,
        )?;
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
            for idx in 0..state.levels.len() {
                self.render_slot(canvas, state, idx)?;
            }
            self.render_selected_count(canvas, state.level_pick.len())?;
            Ok(())
        })?;
        Ok(())
    }

    fn generate_preview<'t>(
        &self,
        texture_creator: &'t TextureCreator<WindowContext>,
        level: &LevelInfo,
    ) -> Result<Option<Texture<'t>>, anyhow::Error> {
        match level {
            LevelInfo::Random => Ok(None),
            LevelInfo::File { map, .. } => Ok(Some(
                map.generate_preview(texture_creator, &self.levels_menu.palette)?,
            )),
        }
    }
}

fn find_levels(path: &Path) -> Result<Vec<LevelInfo>, anyhow::Error> {
    let mut result = Vec::new();
    for entry in path.read_dir()? {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |f| f == "mne" || f == "MNE") {
                let data = std::fs::read(&path)?;
                if let Ok(map) = MapData::from_bytes(data) {
                    let name = path.file_stem().unwrap().to_string_lossy().to_uppercase();
                    result.push(LevelInfo::File { name, map });
                }
            }
        }
    }
    result.push(LevelInfo::Random);
    result.sort_by_cached_key(|v| match v {
        LevelInfo::Random => (false, String::new()),
        LevelInfo::File { name, .. } => (true, name.to_owned()),
    });
    Ok(result)
}
