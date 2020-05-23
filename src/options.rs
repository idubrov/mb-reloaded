use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::Application;
use byteorder::{LittleEndian, ReadBytesExt};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::convert::TryInto;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum WinCondition {
    ByWins,
    ByMoney,
}

#[derive(Debug)]
pub struct Options {
    players: u8,
    treasures: u8,
    rounds: u16,
    cash: u16,
    round_time: Duration,
    // Each point is 3% slowdown from 100%
    // 0 is 100%
    // 33 is 1%
    speed: u16,
    darkness: bool,
    free_market: bool,
    selling: bool,
    win: WinCondition,
    bomb_damage: u8,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            players: 2,
            treasures: 45,
            rounds: 15,
            cash: 750,
            round_time: Duration::from_secs(420),
            speed: 8,
            darkness: false,
            free_market: false,
            selling: false,
            win: WinCondition::ByWins,
            bomb_damage: 100,
        }
    }
}

/// Selected item in the main menu
#[derive(Clone, Copy, PartialEq, PartialOrd, IntoPrimitive, TryFromPrimitive)]
#[repr(usize)]
enum GameOption {
    Cash,
    Treasures,
    Rounds,
    Time,
    Players,
    Speed,
    BombDamage,
    Darkness,
    FreeMarket,
    Selling,
    Winner,
    RedefineKeys,
    LoadLevels,
    MainMenu,
}

impl GameOption {
    /// Get option that is next to the current one
    fn next(self) -> GameOption {
        let pos: usize = self.into();
        let pos: usize = (pos + 1) % usize::from(GameOption::MainMenu);
        pos.try_into().unwrap()
    }

    /// Get game option that is previous to the current one
    fn prev(self) -> GameOption {
        let pos: usize = self.into();
        let pos: usize =
            (pos + usize::from(GameOption::MainMenu)) % (usize::from(GameOption::MainMenu) + 1);
        pos.try_into().unwrap()
    }
}

const OPTIONS_VALUE: [GameOption; 11] = [
    GameOption::Cash,
    GameOption::Treasures,
    GameOption::Rounds,
    GameOption::Time,
    GameOption::Players,
    GameOption::Speed,
    GameOption::BombDamage,
    GameOption::Darkness,
    GameOption::FreeMarket,
    GameOption::Selling,
    GameOption::Winner,
];

impl GameOption {
    /// Left coordinate of the area for the first menu item
    const MENU_ITEM_X: i32 = 192;

    /// Top coordinate of the area for the first menu item
    const MENU_ITEM_Y: i32 = 96;

    /// Option item height
    const ITEM_HEIGHT: i32 = 24;

    /// Position to place the cursor glyph
    fn cursor_pos(self) -> (i32, i32) {
        let y = (self as i32) * Self::ITEM_HEIGHT + Self::MENU_ITEM_Y + 6;
        (Self::MENU_ITEM_X + 25, y)
    }

    /// Rectangle for the bar area
    fn value_bar_rect(self) -> Rect {
        Rect::new(
            Self::MENU_ITEM_X + 142,
            Self::MENU_ITEM_Y + 5 + (self as i32) * Self::ITEM_HEIGHT,
            166,
            13,
        )
    }

    /// Position for the "off" radio button
    fn radio_button_off_pos(self) -> (i32, i32) {
        (
            Self::MENU_ITEM_X + 251,
            Self::MENU_ITEM_Y + 5 + (self as i32) * Self::ITEM_HEIGHT,
        )
    }

    /// Position for the "on" radio button
    fn radio_button_on_pos(self) -> (i32, i32) {
        let x = Self::MENU_ITEM_X + 185;
        let y = Self::MENU_ITEM_Y + 5 + (self as i32) * Self::ITEM_HEIGHT;
        (x, y)
    }

    /// Position to render text
    fn text_pos(self) -> (i32, i32) {
        let x = Self::MENU_ITEM_X + 208;
        let y = Self::MENU_ITEM_Y + 7 + (self as i32) * Self::ITEM_HEIGHT;
        (x, y)
    }
}

impl Options {
    fn from_binary(buf: &[u8]) -> Self {
        assert_eq!(buf.len(), 17);

        let mut it = buf;
        let mut opts = Options {
            players: it.read_u8().unwrap(),
            treasures: it.read_u8().unwrap(),
            rounds: it.read_u16::<LittleEndian>().unwrap(),
            cash: it.read_u16::<LittleEndian>().unwrap(),
            round_time: to_duration(it.read_u32::<LittleEndian>().unwrap()),
            speed: it.read_u16::<LittleEndian>().unwrap(),
            darkness: it.read_u8().unwrap() != 0,
            free_market: it.read_u8().unwrap() != 0,
            selling: it.read_u8().unwrap() != 0,
            win: if it.read_u8().unwrap() != 0 {
                WinCondition::ByWins
            } else {
                WinCondition::ByMoney
            },
            bomb_damage: it.read_u8().unwrap(),
        };
        if opts.players > 4 {
            opts.players = 2;
        }
        if opts.bomb_damage > 100 {
            opts.players = 100;
        }
        if opts.rounds > 55 {
            opts.rounds = 55;
        }
        if opts.treasures > 75 {
            opts.treasures = 75;
        }
        if opts.cash > 2650 {
            opts.cash = 2650;
        }
        if opts.speed > 33 {
            opts.speed = 33;
        }
        opts
    }
}

/// Convert internal representation of time proper duration
fn to_duration(value: u32) -> Duration {
    let seconds = (value as u64) * 151 / 2750;
    Duration::from_secs(seconds)
}

/// Load options from a configuration file. This function uses the same format as the original game.
pub fn load_options(game_dir: &Path) -> Options {
    let path = game_dir.join("options.cfg");
    let mut buf: [u8; 17] = [0; 17];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut buf))
        .map(|()| Options::from_binary(&buf))
        .unwrap_or_default()
}

impl Application {
    pub fn options_menu_loop(&mut self, ctx: &mut ApplicationContext) -> Result<(), anyhow::Error> {
        self.render_options_menu(ctx, GameOption::MainMenu)?;
        ctx.animate(Animation::FadeUp, 7)?;
        self.option_menu_navigation_loop(ctx)?;
        ctx.animate(Animation::FadeDown, 7)?;
        Ok(())
    }

    fn option_menu_navigation_loop(
        &mut self,
        ctx: &mut ApplicationContext,
    ) -> Result<(), anyhow::Error> {
        let mut selected = GameOption::MainMenu;
        loop {
            let key = ctx.wait_key_pressed();
            match key {
                Scancode::Down | Scancode::Kp2 => {
                    let previous = selected;
                    selected = selected.next();
                    self.update_pointer(ctx, previous, selected)?;
                }
                Scancode::Up | Scancode::Kp8 => {
                    let previous = selected;
                    selected = selected.prev();
                    self.update_pointer(ctx, previous, selected)?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn render_options_menu(
        &mut self,
        ctx: &mut ApplicationContext,
        selected: GameOption,
    ) -> Result<(), anyhow::Error> {
        ctx.with_render_context(|canvas| {
            canvas
                .copy(&self.options_menu.texture, None, None)
                .map_err(SdlError)?;
            let (x, y) = selected.cursor_pos();
            self.glyphs.render(canvas, x, y, Glyph::ArrowPointer)?;

            // Render each individual item for the first ten
            for option in &OPTIONS_VALUE {
                self.render_option_value(canvas, *option)?;
            }
            Ok(())
        })?;
        Ok(())
    }

    fn render_option_value(
        &self,
        canvas: &mut WindowCanvas,
        option: GameOption,
    ) -> Result<(), anyhow::Error> {
        if option >= GameOption::Cash && option <= GameOption::BombDamage {
            let rect = option.value_bar_rect();
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.fill_rect(rect).map_err(SdlError)?;
        } else if option >= GameOption::Darkness && option <= GameOption::Winner {
            let enabled = match option {
                GameOption::Darkness => self.options.darkness,
                GameOption::FreeMarket => self.options.free_market,
                GameOption::Selling => self.options.selling,
                GameOption::Winner => self.options.win == WinCondition::ByMoney,
                _ => unreachable!(),
            };
            let glyphs = if enabled {
                [Glyph::RadioOn, Glyph::RadioOff]
            } else {
                [Glyph::RadioOff, Glyph::RadioOn]
            };
            let (x, y) = option.radio_button_on_pos();
            self.glyphs.render(canvas, x, y, glyphs[0])?;
            let (x, y) = option.radio_button_off_pos();
            self.glyphs.render(canvas, x, y, glyphs[1])?;
        }

        // Render values
        if option >= GameOption::Cash && option <= GameOption::BombDamage {
            let value = match option {
                GameOption::Cash => u64::from(self.options.cash) * 165 / 2650,
                GameOption::Treasures => u64::from(self.options.treasures) * 165 / 75,
                GameOption::Rounds => u64::from(self.options.rounds) * 55 / 75,
                GameOption::Time => self.options.round_time.as_secs() * 165 / 1359,
                GameOption::Players => (u64::from(self.options.players) - 1) * 55,
                GameOption::Speed => {
                    let speed = 100 - 3 * u64::from(self.options.speed);
                    speed * 165 / 100
                }
                GameOption::BombDamage => u64::from(self.options.bomb_damage) * 165 / 100,
                _ => 0,
            };
            let mut rect = option.value_bar_rect();
            rect.set_width((value as u32) + 1);
            canvas.set_draw_color(self.options_menu.palette[1]);
            canvas.fill_rect(rect).map_err(SdlError)?;
        }

        // Print text

        let text = match option {
            GameOption::Cash => Some(format!("{}", self.options.cash)),
            GameOption::Treasures => Some(format!("{}", self.options.treasures)),
            GameOption::Time => {
                let seconds = self.options.round_time.as_secs();
                Some(format!("{}:{:02}", seconds / 60, seconds % 60))
            }
            GameOption::Players => Some(format!(" {}", self.options.players)),
            GameOption::Speed => Some(format!(" {}%", 100 - 3 * self.options.speed)),
            GameOption::BombDamage => Some(format!(" {}%", self.options.bomb_damage)),
            _ => None,
        };
        if let Some(text) = text {
            let text_color = self.options_menu.palette[8];
            let (x, y) = option.text_pos();
            self.font.render(canvas, x, y, text_color, &text)?;
        }
        Ok(())
    }

    fn update_pointer(
        &mut self,
        ctx: &mut ApplicationContext,
        previous: GameOption,
        selected: GameOption,
    ) -> Result<(), anyhow::Error> {
        ctx.with_render_context(|canvas| {
            let (old_x, old_y) = previous.cursor_pos();
            let (w, h) = Glyph::ArrowPointer.dimensions();
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas
                .fill_rect(Rect::new(old_x, old_y, w, h))
                .map_err(SdlError)?;
            let (x, y) = selected.cursor_pos();
            self.glyphs.render(canvas, x, y, Glyph::ArrowPointer)?;
            Ok(())
        })?;
        ctx.present()?;
        Ok(())
    }
}
