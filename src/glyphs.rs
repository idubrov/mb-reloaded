use crate::error::ApplicationError::SdlError;
use crate::images::TexturePalette;
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};

/// Glyphs is one single texture with all game icons on it.
pub struct Glyphs<'t> {
  texture: Texture<'t>,
}

/// Type of the glyph that we want to render
#[derive(Clone, Copy)]
pub enum Glyph {
  ShovelPointer,
  ArrowPointer,
  RadioOff,
  RadioOn,
}

impl Glyph {
  /// Get position of the glyph in the texture; these position should correspond to the texture we use.
  fn rect(self) -> Rect {
    let (left, top, right, bottom) = match self {
      Glyph::ShovelPointer => (150, 140, 215, 160),
      Glyph::ArrowPointer => (205, 99, 231, 109),
      Glyph::RadioOff => (90, 40, 104, 52),
      Glyph::RadioOn => (90, 53, 104, 65),
    };
    Rect::new(left, top, (right - left + 1) as u32, (bottom - top + 1) as u32)
  }

  /// Get the dimensions of the glyph (width and height)
  pub fn dimensions(self) -> (u32, u32) {
    let rect = self.rect();
    (rect.width(), rect.height())
  }
}

impl<'t> Glyphs<'t> {
  /// Load glyph texture
  pub fn from_texture(texture: TexturePalette<'t>) -> Glyphs<'t> {
    Self {
      texture: texture.texture,
    }
  }

  /// Render given glyph at position
  pub fn render(&self, canvas: &mut WindowCanvas, x: i32, y: i32, glyph: Glyph) -> Result<(), anyhow::Error> {
    let src_rect = glyph.rect();
    let tgt_rect = Rect::new(x, y, src_rect.width(), src_rect.height());
    canvas.copy(&self.texture, src_rect, tgt_rect).map_err(SdlError)?;
    Ok(())
  }
}
