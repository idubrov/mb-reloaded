use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::keys::Key;
use crate::Application;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::convert::TryInto;

impl Application<'_> {
  pub fn redefine_keys_menu(&mut self, ctx: &mut ApplicationContext) -> Result<(), anyhow::Error> {
    ctx.with_render_context(|canvas| {
      canvas.copy(&self.keys.texture, None, None).map_err(SdlError)?;
      self.render_configured_keys(canvas)?;
      Ok(())
    })?;
    ctx.animate(Animation::FadeUp, 7)?;

    let color = self.keys.palette[5];
    'outer: for player in 0..4 {
      for key in Key::all_keys() {
        let (scan, _) = ctx.wait_key_pressed();
        if scan == Scancode::F10 {
          break 'outer;
        }
        if scan != Scancode::Escape {
          self.player_keys[player][key] = Some(scan);
        }

        // Re-render the key
        ctx.with_render_context(|canvas| {
          if let Some(scancode) = self.player_keys[player][key] {
            let y = key_pos_y(player, key);

            canvas.set_draw_color(Color::BLACK);
            let rect = Rect::new(356, y, 144, 8);
            canvas.fill_rect(rect).map_err(SdlError)?;
            self
              .font
              .render(canvas, 356, y, color, &scancode.name().to_uppercase())?;
          }
          Ok(())
        })?;
        ctx.present()?;
      }
    }

    // Save all assigned keys
    crate::keys::save_keys(&self.player_keys, ctx.game_dir())?;
    ctx.animate(Animation::FadeDown, 7)?;
    Ok(())
  }

  fn render_configured_keys(&self, canvas: &mut WindowCanvas) -> Result<(), anyhow::Error> {
    const COLORS: [usize; 3] = [12, 4, 8];
    const OFFSETS: [i32; 3] = [-1, 1, 0];
    for player in 0..4 {
      for layer in 0..3 {
        for key in 0..8 {
          let key: Key = key.try_into().unwrap();
          let keys = &self.player_keys[player as usize];
          let color = self.keys.palette[COLORS[layer]];

          let y = key_pos_y(player, key);
          let text = format!("Player {} {:11}: ", player + 1, key.to_string());
          self.font.render(canvas, 180 + OFFSETS[layer], y, color, &text)?;

          // Don't render "shadow" for keys
          if layer == 2 {
            if let Some(scancode) = keys[key] {
              self
                .font
                .render(canvas, 356, y, color, &scancode.name().to_uppercase())?;
            }
          }
        }
      }
    }
    Ok(())
  }
}

fn key_pos_y(player: usize, key: Key) -> i32 {
  (player as i32) * 80 + 10 * (key as i32) + 100
}
