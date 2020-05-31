use crate::entity::Equipment;
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
  Radio(bool),
  ShopSlot(bool),
  Selection(Equipment),
  Ready,
}

impl Glyph {
  /// Get position of the glyph in the texture; these position should correspond to the texture we use.
  fn rect(self) -> Rect {
    let (left, top, right, bottom) = match self {
      Glyph::ShovelPointer => (150, 140, 215, 160),
      Glyph::ArrowPointer => (205, 99, 231, 109),
      Glyph::Radio(false) => (90, 40, 104, 52),
      Glyph::Radio(true) => (90, 53, 104, 65),
      Glyph::ShopSlot(false) => (64, 92, 127, 139),
      Glyph::ShopSlot(true) => (128, 92, 191, 139),
      Glyph::Selection(Equipment::SmallBomb) => (0, 170, 29, 199),
      Glyph::Selection(Equipment::LargeBomb) => (30, 170, 59, 199),
      Glyph::Selection(Equipment::Dynamite) => (60, 170, 89, 199),
      Glyph::Selection(Equipment::AtomicBomb) => (216, 140, 245, 169),
      Glyph::Selection(Equipment::SmallRadio) => (240, 170, 269, 199),
      Glyph::Selection(Equipment::LargeRadio) => (210, 170, 239, 199),
      Glyph::Selection(Equipment::Grenade) => (246, 140, 275, 169),
      Glyph::Selection(Equipment::Mine) => (270, 170, 299, 199),
      Glyph::Selection(Equipment::Flamethrower) => (90, 170, 119, 199),
      Glyph::Selection(Equipment::Napalm) => (120, 170, 149, 199),
      Glyph::Selection(Equipment::Barrel) => (246, 110, 275, 139),
      Glyph::Selection(Equipment::SmallCrucifix) => (90, 140, 119, 169),
      Glyph::Selection(Equipment::LargeCrucifix) => (150, 170, 179, 199),
      Glyph::Selection(Equipment::Plastic) => (180, 170, 209, 199),
      Glyph::Selection(Equipment::ExplosivePlastic) => (276, 140, 305, 169),
      Glyph::Selection(Equipment::Digger) => (276, 110, 305, 139),
      Glyph::Selection(Equipment::MetalWall) => (216, 110, 245, 139),
      Glyph::Selection(Equipment::SmallPickaxe) => (0, 140, 29, 169),
      Glyph::Selection(Equipment::LargePickaxe) => (30, 140, 59, 169),
      Glyph::Selection(Equipment::Drill) => (60, 140, 89, 169),
      Glyph::Selection(Equipment::Teleport) => (30, 40, 59, 69),
      Glyph::Selection(Equipment::Clone) => (232, 80, 261, 109),
      Glyph::Selection(Equipment::Biomass) => (262, 80, 291, 109),
      Glyph::Selection(Equipment::Extinguisher) => (0, 40, 29, 69),
      Glyph::Selection(Equipment::Armor) => (105, 40, 134, 69),
      Glyph::Selection(Equipment::JumpingBomb) => (60, 40, 89, 69),
      Glyph::Selection(Equipment::SuperDrill) => (0, 90, 29, 119),
      Glyph::Ready => (120, 140, 149, 169),
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
