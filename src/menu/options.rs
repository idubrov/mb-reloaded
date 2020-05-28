use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::options::{Options, WinCondition};
use crate::settings::GameSettings;
use crate::Application;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::convert::TryInto;
use std::time::Duration;

/// Items in the options menu. Note that ordering must match the texture used for the menu.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, IntoPrimitive, TryFromPrimitive)]
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
    let pos: usize = (pos + 1) % (usize::from(GameOption::MainMenu) + 1);
    pos.try_into().unwrap()
  }

  /// Get game option that is previous to the current one
  fn prev(self) -> GameOption {
    let pos: usize = self.into();
    let pos: usize = (pos + usize::from(GameOption::MainMenu)) % (usize::from(GameOption::MainMenu) + 1);
    pos.try_into().unwrap()
  }

  /// Iterate through all of the options in the options menu. Note that it also includes
  /// items for other menus (redefine keys, load levels and main menu).
  fn all_options() -> impl Iterator<Item = GameOption> {
    (0..14).map(|v| v.try_into().unwrap())
  }

  fn value_minus(self, options: &mut Options) {
    match self {
      GameOption::Cash => {
        if options.cash >= 100 {
          options.cash -= 100;
        } else {
          options.cash = 0;
        }
      }
      GameOption::Treasures if options.treasures > 0 => {
        options.treasures -= 1;
      }
      GameOption::Rounds if options.rounds > 1 => {
        options.rounds -= 1;
      }
      GameOption::Time => {
        options.round_time = options
          .round_time
          .checked_sub(Duration::from_secs(15))
          .unwrap_or(Duration::from_secs(0));
      }
      GameOption::Players if options.players > 1 => {
        options.players -= 1;
      }
      GameOption::Speed if options.speed < 33 => {
        options.speed += 1;
      }
      GameOption::BombDamage if options.bomb_damage > 0 => {
        options.bomb_damage -= 1;
      }
      GameOption::Darkness => {
        options.darkness = !options.darkness;
      }
      GameOption::FreeMarket => {
        options.free_market = !options.free_market;
      }
      GameOption::Selling => {
        options.selling = !options.selling;
      }
      GameOption::Winner if options.win == WinCondition::ByWins => {
        options.win = WinCondition::ByMoney;
      }
      GameOption::Winner if options.win == WinCondition::ByMoney => {
        options.win = WinCondition::ByWins;
      }
      _ => {}
    }
  }

  fn value_plus(self, options: &mut Options) {
    match self {
      GameOption::Cash => {
        options.cash += 100;
        if options.cash > 2650 {
          options.cash = 2650;
        }
      }
      GameOption::Treasures if options.treasures < 75 => {
        options.treasures += 1;
      }
      GameOption::Rounds if options.rounds < 55 => {
        options.rounds += 1;
      }
      GameOption::Time => {
        options.round_time += Duration::from_secs(15);
        if options.round_time > Duration::from_secs(22 * 60 + 40) {
          options.round_time = Duration::from_secs(22 * 60 + 40)
        }
      }
      GameOption::Players if options.players < 4 => {
        options.players += 1;
      }
      GameOption::Speed if options.speed > 0 => {
        options.speed -= 1;
      }
      GameOption::BombDamage if options.bomb_damage < 100 => {
        options.bomb_damage += 1;
      }
      GameOption::Darkness => {
        options.darkness = !options.darkness;
      }
      GameOption::FreeMarket => {
        options.free_market = !options.free_market;
      }
      GameOption::Selling => {
        options.selling = !options.selling;
      }
      GameOption::Winner if options.win == WinCondition::ByWins => {
        options.win = WinCondition::ByMoney;
      }
      GameOption::Winner if options.win == WinCondition::ByMoney => {
        options.win = WinCondition::ByWins;
      }
      _ => {}
    }
  }
}

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

impl Application<'_> {
  pub fn options_menu(&self, ctx: &mut ApplicationContext, settings: &mut GameSettings) -> Result<(), anyhow::Error> {
    loop {
      self.render_options_menu(ctx, &settings.options, GameOption::MainMenu)?;
      ctx.animate(Animation::FadeUp, 7)?;
      let selected = self.option_menu_navigation_loop(ctx, &mut settings.options)?;
      ctx.animate(Animation::FadeDown, 7)?;

      match selected {
        GameOption::LoadLevels => {
          settings.levels = self.load_levels(ctx, usize::from(settings.options.rounds))?;
        }
        GameOption::RedefineKeys => {
          self.redefine_keys_menu(ctx, &mut settings.keys)?;
        }
        GameOption::MainMenu => break,
        // Should never get here
        _ => {}
      }
    }

    // Save options
    settings.options.save(ctx.game_dir())?;
    Ok(())
  }

  fn option_menu_navigation_loop(
    &self,
    ctx: &mut ApplicationContext,
    options: &mut Options,
  ) -> Result<GameOption, anyhow::Error> {
    let mut selected = GameOption::MainMenu;
    loop {
      let (scancode, keycode) = ctx.wait_key_pressed();
      match scancode {
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
        Scancode::Escape => {
          return Ok(GameOption::MainMenu);
        }
        Scancode::Return | Scancode::KpEnter
          if selected == GameOption::RedefineKeys
            || selected == GameOption::LoadLevels
            || selected == GameOption::MainMenu =>
        {
          return Ok(selected);
        }
        Scancode::Left => {
          selected.value_minus(options);
          ctx.with_render_context(|canvas| {
            self.render_option_value(canvas, options, selected)?;
            Ok(())
          })?;
          ctx.present()?;
        }
        Scancode::Right => {
          selected.value_plus(options);
          ctx.with_render_context(|canvas| {
            self.render_option_value(canvas, options, selected)?;
            Ok(())
          })?;
          ctx.present()?;
        }
        Scancode::Return | Scancode::KpEnter if selected == GameOption::RedefineKeys => {
          panic!();
          // ctx.animate(Animation::FadeDown, 7)?;
          // self.redefine_keys_menu(ctx, &mut settings.keys)?;
          // ctx.animate(Animation::FadeUp, 7)?;
        }
        _ if keycode == Keycode::D => {
          *options = Options::default();
          ctx.with_render_context(|canvas| {
            for option in GameOption::all_options() {
              self.render_option_value(canvas, options, option)?;
            }
            Ok(())
          })?;
          ctx.present()?;
        }
        _ => {}
      }
    }
  }

  fn render_options_menu(
    &self,
    ctx: &mut ApplicationContext,
    options: &Options,
    selected: GameOption,
  ) -> Result<(), anyhow::Error> {
    ctx.with_render_context(|canvas| {
      canvas.copy(&self.options_menu.texture, None, None).map_err(SdlError)?;
      let (x, y) = selected.cursor_pos();
      self.glyphs.render(canvas, x, y, Glyph::ArrowPointer)?;

      for option in GameOption::all_options() {
        self.render_option_value(canvas, options, option)?;
      }
      Ok(())
    })?;
    Ok(())
  }

  /// Render value for the given option
  fn render_option_value(
    &self,
    canvas: &mut WindowCanvas,
    options: &Options,
    option: GameOption,
  ) -> Result<(), anyhow::Error> {
    if option >= GameOption::Cash && option <= GameOption::BombDamage {
      let rect = option.value_bar_rect();
      canvas.set_draw_color(Color::RGB(0, 0, 0));
      canvas.fill_rect(rect).map_err(SdlError)?;
    } else if option >= GameOption::Darkness && option <= GameOption::Winner {
      let enabled = match option {
        GameOption::Darkness => options.darkness,
        GameOption::FreeMarket => options.free_market,
        GameOption::Selling => options.selling,
        GameOption::Winner => options.win == WinCondition::ByMoney,
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
        GameOption::Cash => u64::from(options.cash) * 165 / 2650,
        GameOption::Treasures => u64::from(options.treasures) * 165 / 75,
        GameOption::Rounds => u64::from(options.rounds) * 165 / 55,
        GameOption::Time => options.round_time.as_secs() * 165 / 1359,
        GameOption::Players => (u64::from(options.players) - 1) * 55,
        GameOption::Speed => {
          let speed = 100 - 3 * u64::from(options.speed);
          speed * 165 / 100
        }
        GameOption::BombDamage => u64::from(options.bomb_damage) * 165 / 100,
        _ => 0,
      };
      let mut rect = option.value_bar_rect();
      rect.set_width((value as u32) + 1);
      canvas.set_draw_color(self.options_menu.palette[1]);
      canvas.fill_rect(rect).map_err(SdlError)?;
    }

    // Print text

    let text = match option {
      GameOption::Cash => Some(format!("{}", options.cash)),
      GameOption::Treasures => Some(format!("{}", options.treasures)),
      GameOption::Rounds => Some(format!("{}", options.rounds)),
      GameOption::Time => {
        let seconds = options.round_time.as_secs();
        Some(format!("{}:{:02} min", seconds / 60, seconds % 60))
      }
      GameOption::Players => Some(format!(" {}", options.players)),
      GameOption::Speed => Some(format!(" {}%", 100 - 3 * options.speed)),
      GameOption::BombDamage => Some(format!(" {}%", options.bomb_damage)),
      _ => None,
    };
    if let Some(text) = text {
      let text_color = self.options_menu.palette[8];
      let (x, y) = option.text_pos();
      self.font.render(canvas, x, y, text_color, &text)?;
    }
    Ok(())
  }

  /// Update cursor icon
  fn update_pointer(
    &self,
    ctx: &mut ApplicationContext,
    previous: GameOption,
    selected: GameOption,
  ) -> Result<(), anyhow::Error> {
    ctx.with_render_context(|canvas| {
      let (old_x, old_y) = previous.cursor_pos();
      let (w, h) = Glyph::ArrowPointer.dimensions();
      canvas.set_draw_color(Color::RGB(0, 0, 0));
      canvas.fill_rect(Rect::new(old_x, old_y, w, h)).map_err(SdlError)?;
      let (x, y) = selected.cursor_pos();
      self.glyphs.render(canvas, x, y, Glyph::ArrowPointer)?;
      Ok(())
    })?;
    ctx.present()?;
    Ok(())
  }
}
