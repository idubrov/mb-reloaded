use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::settings::GameSettings;
use crate::Application;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

/// Selected item in the main menu
#[derive(Clone, Copy, PartialEq)]
#[repr(usize)]
enum SelectedMenu {
  NewGame,
  Options,
  Info,
  Quit,
}

impl SelectedMenu {
  /// Get menu item that is next to the current one
  fn next(self) -> SelectedMenu {
    match self {
      SelectedMenu::NewGame => SelectedMenu::Options,
      SelectedMenu::Options => SelectedMenu::Info,
      SelectedMenu::Info => SelectedMenu::Quit,
      SelectedMenu::Quit => SelectedMenu::NewGame,
    }
  }

  /// Get menu item that is previous to the current one
  fn prev(self) -> SelectedMenu {
    self.next().next().next()
  }

  /// Shovel position in the main menu based on the selected item. Should correspond to the main menu texture.
  fn shovel_pos(self) -> (i32, i32) {
    (222, 136 + 48 * self as i32)
  }
}

impl Application<'_> {
  pub fn main_menu(self, ctx: &mut ApplicationContext, campaign_mode: bool) -> Result<(), anyhow::Error> {
    self.music1.play(-1).map_err(SdlError)?;

    ctx.render_texture(&self.title.texture)?;
    ctx.animate(Animation::FadeUp, 7)?;
    let (scancode, _) = ctx.wait_key_pressed();
    ctx.animate(Animation::FadeDown, 7)?;
    if scancode == Scancode::Escape {
      return Ok(());
    }

    self.main_menu_loop(ctx, campaign_mode)?;
    Ok(())
  }

  /// Returns when exiting the game
  fn main_menu_loop(&self, ctx: &mut ApplicationContext, campaign_mode: bool) -> Result<(), anyhow::Error> {
    let mut settings = GameSettings::load(ctx.game_dir());
    settings.options.campaign_mode = campaign_mode;

    let mut selected_item = SelectedMenu::NewGame;
    loop {
      self.render_main_menu(ctx, selected_item)?;
      ctx.animate(Animation::FadeUp, 7)?;
      self.main_menu_navigation_loop(ctx, &mut selected_item)?;
      ctx.animate(Animation::FadeDown, 7)?;
      match selected_item {
        SelectedMenu::Quit => break Ok(()),
        SelectedMenu::NewGame => {
          self.play_game(ctx, &settings)?;
          self.music1.play(-1).map_err(SdlError)?;
        }
        SelectedMenu::Options => self.options_menu(ctx, &mut settings)?,
        SelectedMenu::Info => self.info_menu(ctx)?,
      }
    }
  }

  /// Runs navigation inside main menu. Return
  fn main_menu_navigation_loop(
    &self,
    ctx: &mut ApplicationContext,
    selected: &mut SelectedMenu,
  ) -> Result<(), anyhow::Error> {
    loop {
      let (scancode, _keycode) = ctx.wait_key_pressed();

      match scancode {
        Scancode::Down | Scancode::Kp2 => {
          let next = selected.next();
          self.update_shovel(ctx, *selected, next)?;
          *selected = next;
        }
        Scancode::Up | Scancode::Kp8 => {
          let prev = selected.prev();
          self.update_shovel(ctx, *selected, prev)?;
          *selected = prev;
        }
        Scancode::Escape => {
          *selected = SelectedMenu::Quit;
          break;
        }
        Scancode::Kp3 | Scancode::Return | Scancode::Return2 | Scancode::KpEnter => {
          break;
        }
        _ => {}
      }
    }
    Ok(())
  }

  /// Display main menu with selected option, plus animation
  fn render_main_menu(&self, ctx: &mut ApplicationContext, selected: SelectedMenu) -> Result<(), anyhow::Error> {
    let texture = &self.main_menu;
    let glyphs = &self.glyphs;
    ctx.with_render_context(|canvas| {
      canvas.copy(&texture.texture, None, None).map_err(SdlError)?;

      // Render "Registered to"
      let pos = ((26 - self.registered.len()) * 4 + 254) as i32;
      let palette = &self.main_menu.palette;
      self.font.render(canvas, pos - 1, 437, palette[10], &self.registered)?;
      self.font.render(canvas, pos + 1, 437, palette[8], &self.registered)?;
      self.font.render(canvas, pos, 437, palette[0], &self.registered)?;

      let (x, y) = selected.shovel_pos();
      glyphs.render(canvas, x, y, Glyph::ShovelPointer)?;
      Ok(())
    })?;
    Ok(())
  }

  fn update_shovel(
    &self,
    ctx: &mut ApplicationContext,
    previous: SelectedMenu,
    selected: SelectedMenu,
  ) -> Result<(), anyhow::Error> {
    ctx.with_render_context(|canvas| {
      let (old_x, old_y) = previous.shovel_pos();
      let (w, h) = Glyph::ShovelPointer.dimensions();
      canvas.set_draw_color(Color::RGB(0, 0, 0));
      canvas.fill_rect(Rect::new(old_x, old_y, w, h)).map_err(SdlError)?;
      let (x, y) = selected.shovel_pos();
      self.glyphs.render(canvas, x, y, Glyph::ShovelPointer)?;
      Ok(())
    })?;
    ctx.present()?;
    Ok(())
  }

  fn info_menu(&self, ctx: &mut ApplicationContext) -> Result<(), anyhow::Error> {
    let mut key = Scancode::Escape;
    for info in &self.info {
      ctx.render_texture(&info.texture)?;
      ctx.animate(Animation::FadeUp, 7)?;
      key = ctx.wait_key_pressed().0;
      ctx.animate(Animation::FadeDown, 7)?;
      if key == Scancode::Escape {
        break;
      }
    }
    if key == Scancode::Tab {
      ctx.render_texture(&self.codes.texture)?;
      ctx.animate(Animation::FadeUp, 7)?;
      ctx.wait_key_pressed();
      ctx.animate(Animation::FadeDown, 7)?;
    }
    Ok(())
  }
}
