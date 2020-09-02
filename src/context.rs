use crate::error::ApplicationError::SdlError;
use crate::fonts::Font;
use crate::images::{TextureFormat, TexturePalette};
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::mixer::{Music, AUDIO_S16LSB};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use sdl2::EventPump;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Application environment resources packaged into one structs. Provides helper functions used
/// across the whole application.
pub struct ApplicationContext<'canvas, 'textures> {
  game_dir: PathBuf,
  events: EventPump,
  canvas: &'canvas mut WindowCanvas,
  buffer: Texture<'textures>,
  texture_creator: &'textures TextureCreator<WindowContext>,
}

pub enum Animation {
  FadeUp,
  FadeDown,
}

/// Our representation of an input event. Most of the time, we only care about scancodes. However,
/// when we allow entering a text (player creation screen), we need to be able to represent input
/// text as well.
pub enum InputEvent {
  KeyPress(Scancode, Keycode),
  TextInput(String),
}

impl<'canvas, 'textures> ApplicationContext<'canvas, 'textures> {
  pub fn with_context(
    game_dir: PathBuf,
    cb: impl FnOnce(ApplicationContext) -> Result<(), anyhow::Error>,
  ) -> Result<(), anyhow::Error> {
    let sdl_context = sdl2::init().map_err(SdlError)?;
    let video = sdl_context.video().map_err(SdlError)?;
    let window = video
      .window("MineBombers Reloaded", SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
      .position_centered()
      .allow_highdpi()
      .resizable()
      .build()?;
    let mut canvas = window.into_canvas().build()?;
    let events = sdl_context.event_pump().map_err(SdlError)?;
    let texture_creator = canvas.texture_creator();

    // Create texture we use as a permanent buffer for rendering, to make it easier to
    // replicate original game (this buffer is an equivalent of "video buffer").
    // This allows us to do additive rendering and do a "pallette animation" by blending it
    // with an alpha modifier on top of black screen.
    let buffer =
      texture_creator.create_texture_target(PixelFormatEnum::RGB24, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)?;

    // Initialize audio
    sdl2::mixer::open_audio(44100, AUDIO_S16LSB, 2, 1024).map_err(SdlError)?;
    let ctx = ApplicationContext {
      game_dir,
      canvas: &mut canvas,
      events,
      buffer,
      texture_creator: &texture_creator,
    };
    cb(ctx)?;
    Ok(())
  }

  /// Invoke callback in a "rendering" context. Makes canvas to render in a separate buffer
  /// texture so we can apply post-processing to it (for example, emulate palette animation).
  pub fn with_render_context<R>(
    &mut self,
    callback: impl FnOnce(&mut WindowCanvas) -> Result<R, anyhow::Error>,
  ) -> Result<R, anyhow::Error> {
    let mut result = None;
    self.canvas.with_texture_canvas(&mut self.buffer, |canvas| {
      result = Some(callback(canvas));
    })?;
    result.unwrap()
  }

  pub fn render_texture(&mut self, texture: &Texture) -> Result<(), anyhow::Error> {
    self.with_render_context(|canvas| {
      canvas.copy(&texture, None, None).map_err(SdlError)?;
      Ok(())
    })?;
    Ok(())
  }

  /// Load SPY texture from a given path
  pub fn load_spy(&self, file_name: &str) -> Result<TexturePalette<'textures>, anyhow::Error> {
    let path = self.game_dir.join(file_name);
    Ok(crate::images::load_texture(
      &self.texture_creator,
      &path,
      TextureFormat::SPY,
    )?)
  }

  /// Load PPM texture from a given path
  pub fn load_ppm(&self, file_name: &str) -> Result<TexturePalette<'textures>, anyhow::Error> {
    let path = self.game_dir.join(file_name);
    Ok(crate::images::load_texture(
      &self.texture_creator,
      &path,
      TextureFormat::PPM,
    )?)
  }

  /// Load fonts from a given path
  pub fn load_font(&self, file_name: &str) -> Result<Font<'textures>, anyhow::Error> {
    let path = self.game_dir.join(file_name);
    Ok(crate::fonts::load_font(&self.texture_creator, &path)?)
  }

  pub fn load_music(&self, file_name: &str) -> Result<Music<'static>, anyhow::Error> {
    let path = self.game_dir.join(file_name);
    let music = Music::from_file(path).map_err(SdlError)?;
    Ok(music)
  }

  pub fn animate(&mut self, animation: Animation, steps: usize) -> Result<(), anyhow::Error> {
    // Note that we actually do steps + 1 iteration, as per original behavior
    // Roughly, we do it for half a second for 8 steps. For 60 FPS, which means ~4 frames per step.
    let total_frames = (steps + 1) * 4;

    for idx in 0..=total_frames {
      self.canvas.set_draw_color(Color::RGB(0, 0, 0));
      self.canvas.clear();
      let mut alpha = (255 * idx / total_frames) as u8;
      if let Animation::FadeDown = animation {
        alpha = 255 - alpha;
      }
      self.buffer.set_blend_mode(BlendMode::Blend);
      self.buffer.set_alpha_mod(alpha);
      self.canvas.copy(&self.buffer, None, None).map_err(SdlError)?;

      self.events.pump_events();
      self.canvas.present();
      self.wait_frame();
    }
    Ok(())
  }

  pub fn present(&mut self) -> Result<(), anyhow::Error> {
    self.present_shake(0)
  }

  pub fn present_shake(&mut self, shake: u16) -> Result<(), anyhow::Error> {
    self.buffer.set_blend_mode(BlendMode::None);
    self.buffer.set_alpha_mod(255);
    let (w, h) = self.canvas.output_size().map_err(SdlError)?;
    let mut target = Rect::new(0, 0, w, h);

    // Render "shaking" screen effect
    if shake != 0 {
      let top = u32::from(shake) * 10 * h / SCREEN_HEIGHT;
      target.set_y(-(top as i32));
      self.canvas.set_draw_color(Color::BLACK);
      self
        .canvas
        .fill_rect(Rect::new(0, (h - top) as i32, w, top))
        .map_err(SdlError)?;
    }
    self.canvas.copy(&self.buffer, None, Some(target)).map_err(SdlError)?;
    self.canvas.present();
    Ok(())
  }

  pub fn present_flash(&mut self) -> Result<(), anyhow::Error> {
    self.canvas.set_draw_color(Color::WHITE);
    self.canvas.clear();
    self.canvas.present();
    Ok(())
  }

  pub fn wait_frame(&self) {
    // We should wait for the remaining time; for now just do a fixed delay.
    ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
  }

  /// Wait until some key is pressed
  pub fn wait_input_event(&mut self) -> InputEvent {
    loop {
      let event = self.events.wait_event();
      match event {
        // FIXME: proper event
        Event::Quit { .. } => return InputEvent::KeyPress(Scancode::Escape, Keycode::Escape),
        Event::KeyDown {
          scancode: Some(code),
          keycode: Some(key),
          ..
        } => return InputEvent::KeyPress(code, key),
        Event::TextInput { text, .. } => return InputEvent::TextInput(text),
        _ => {}
      }
    }
  }

  /// Wait until some key is pressed. This is a simpler interface for cases where we don't expect
  /// text input (most of the time, we don't)
  pub fn wait_key_pressed(&mut self) -> (Scancode, Keycode) {
    loop {
      if let InputEvent::KeyPress(scan, key) = self.wait_input_event() {
        return (scan, key);
      }
    }
  }

  pub fn poll_iter<'ev>(&'ev mut self) -> impl Iterator<Item = Event> + 'ev {
    self.events.poll_iter()
  }

  pub fn game_dir(&self) -> &Path {
    &self.game_dir
  }

  pub fn texture_creator(&self) -> &'textures TextureCreator<WindowContext> {
    self.texture_creator
  }
}
