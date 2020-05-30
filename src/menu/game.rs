use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::map::{LevelInfo, LevelMap};
use crate::player::PlayerEntity;
use crate::settings::GameSettings;
use crate::Application;
use sdl2::pixels::Color;
use std::rc::Rc;

impl Application<'_> {
  /// Play game, starting from player selection
  pub fn play_game(&self, ctx: &mut ApplicationContext, settings: &GameSettings) -> Result<(), anyhow::Error> {
    sdl2::mixer::Music::halt();
    let selected = self.players_select_menu(ctx, settings.options.players)?;
    if selected.is_empty() {
      return Ok(());
    }

    let mut entities = Vec::with_capacity(selected.len());
    for (idx, selected) in selected.into_iter().enumerate() {
      entities.push(PlayerEntity::new(
        selected,
        settings.keys.keys[idx],
        u32::from(settings.options.cash),
      ));
    }

    for round in 0..settings.options.rounds {
      ctx.with_render_context(|canvas| {
        canvas.set_draw_color(Color::BLACK);
        canvas.clear();
        let color = self.main_menu.palette[1];
        self
          .font
          .render(canvas, 220, 200, color, "Creating level...please wait")?;
        Ok(())
      })?;

      // Generate level if necessary
      ctx.animate(Animation::FadeUp, 7)?;
      let level = settings
        .levels
        .get(usize::from(round))
        .map(Rc::as_ref)
        .unwrap_or(&LevelInfo::Random);
      ctx.animate(Animation::FadeDown, 7)?;

      if self.play_round(ctx, &mut entities, round, level, settings)? {
        break;
      }
    }
    Ok(())
  }

  pub fn play_round(
    &self,
    ctx: &mut ApplicationContext,
    players: &mut [PlayerEntity],
    round: u16,
    level: &LevelInfo,
    settings: &GameSettings,
  ) -> Result<bool, anyhow::Error> {
    let level = match level {
      LevelInfo::Random => {
        let mut level = LevelMap::random_map(settings.options.treasures);
        level.generate_entrances(settings.options.players);
        level
      }
      LevelInfo::File { map, .. } => map.clone(),
    };

    // FIXME: generate monsters list
    // Play shop music
    self.music2.play(-1).map_err(SdlError)?;
    sdl2::mixer::Music::set_pos(464.8).map_err(SdlError)?;

    let mut it = players.iter_mut();

    while let Some(right) = it.next() {
      let left = it.next();
      let remaining = settings.options.rounds - round;
      let preview_map = if settings.options.darkness { None } else { Some(&level) };
      self.shop(ctx, remaining, &settings.options, preview_map, left, right)?;
    }

    ctx.wait_input_event();
    Ok(true)
  }
}
