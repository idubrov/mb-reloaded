use crate::context::ApplicationContext;
use crate::error::ApplicationError::SdlError;
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};

/// Glyphs is one single texture with all game icons on it.
pub struct Glyphs {
    texture: Texture,
}

/// Type of the glyph that we want to render
pub enum Glyph {
    Shovel,
}

impl Glyph {
    /// Get position of the glyph in the texture; these position should correspond to the texture we use.
    fn rect(&self) -> Rect {
        let (left, top, right, bottom) = match self {
            Glyph::Shovel => (150, 140, 215, 160),
        };
        Rect::new(
            left,
            top,
            (right - left + 1) as u32,
            (bottom - top + 1) as u32,
        )
    }

    /// Get the dimensions of the glyph (width and height)
    pub fn dimensions(&self) -> (u32, u32) {
        let rect = self.rect();
        (rect.width(), rect.height())
    }
}

impl Glyphs {
    /// Load glyph texture
    pub fn load(context: &ApplicationContext) -> Result<Glyphs, anyhow::Error> {
        let texture = context.load_texture("sika.spy")?;
        Ok(Self { texture })
    }

    /// Render given glyph at position
    pub fn render(
        &self,
        canvas: &mut WindowCanvas,
        glyph: Glyph,
        x: i32,
        y: i32,
    ) -> Result<(), anyhow::Error> {
        let src_rect = glyph.rect();
        let tgt_rect = Rect::new(x, y, src_rect.width(), src_rect.height());
        canvas
            .copy(&self.texture, src_rect, tgt_rect)
            .map_err(SdlError)?;
        Ok(())
    }
}
