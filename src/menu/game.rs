use crate::context::ApplicationContext;
use crate::settings::GameSettings;
use crate::Application;
use sdl2::pixels::Color;

impl Application<'_> {
  /// Play game, starting from player selection
  pub fn play_game(&self, ctx: &mut ApplicationContext, settings: &GameSettings) -> Result<(), anyhow::Error> {
    sdl2::mixer::Music::halt();
    let _players = self.players_select_menu(ctx, settings.options.players)?;

    ctx.with_render_context(|canvas| {
      canvas.set_draw_color(Color::BLACK);
      canvas.clear();
      let color = self.main_menu.palette[1];
      self
        .font
        .render(canvas, 220, 200, color, "Creating level...please wait")?;
      Ok(())
    })?;
    ctx.present()?;
    Ok(())
  }
}
