use crate::error::ApplicationError::SdlError;
use crate::images::TexturePalette;
use crate::world::actor::ActorKind;
use crate::world::equipment::Equipment;
use crate::world::map::MapValue;
use crate::world::position::Direction;
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};

/// Glyphs is one single texture with all game icons on it.
pub struct Glyphs<'t> {
  texture: Texture<'t>,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationPhase {
  Phase1 = 0,
  Phase2 = 1,
  Phase3 = 2,
  Phase4 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Digging {
  Hands,
  Pickaxe,
}

/// Type of the glyph that we want to render
#[derive(Clone, Copy)]
pub enum Glyph {
  ShovelPointer,
  ArrowPointer,
  RadioButton(bool),
  ShopSlot(bool),
  Selection(Equipment),
  Ready,
  // Glyph used to render map cell; note that not all of the glyph actually have an image
  Map(MapValue),
  SandBorder(Direction),
  StoneBorder(Direction),
  BurnedBorder(Direction),
  Monster(ActorKind, Direction, Digging, AnimationPhase),
}

impl Glyph {
  /// Get position of the glyph in the texture; these position should correspond to the texture we use.
  fn rect(self) -> Rect {
    let (left, top, right, bottom) = match self {
      Glyph::ShovelPointer => (150, 140, 215, 160),
      Glyph::ArrowPointer => (205, 99, 231, 109),
      Glyph::RadioButton(false) => (90, 40, 104, 52),
      Glyph::RadioButton(true) => (90, 53, 104, 65),
      Glyph::ShopSlot(false) => (64, 92, 127, 139),
      Glyph::ShopSlot(true) => (128, 92, 191, 139),
      Glyph::Ready => (120, 140, 149, 169),
      Glyph::Selection(equpment) => {
        let (x, y) = EQUIPMENT_GLYPHS[equpment as usize];
        (x, y, x + 29, y + 29)
      }
      Glyph::Map(value) => {
        let (x, y) = if value >= MapValue::Passage && value <= MapValue::Item182 {
          MAP_GLYPHS[(value as usize) - (MapValue::Passage as usize)]
        } else {
          UNMAPPED
        };
        (x, y, x + 9, y + 9)
      }
      Glyph::SandBorder(Direction::Left) => (194, 98, 197, 107),
      Glyph::SandBorder(Direction::Right) => (200, 98, 203, 107),
      Glyph::SandBorder(Direction::Up) => (194, 109, 203, 111),
      Glyph::SandBorder(Direction::Down) => (194, 113, 203, 115),

      Glyph::StoneBorder(Direction::Left) => (148, 60, 151, 69),
      Glyph::StoneBorder(Direction::Right) => (154, 60, 157, 69),
      Glyph::StoneBorder(Direction::Up) => (148, 71, 157, 73),
      Glyph::StoneBorder(Direction::Down) => (148, 75, 157, 77),

      Glyph::BurnedBorder(Direction::Left) => (194, 117, 197, 126),
      Glyph::BurnedBorder(Direction::Right) => (200, 117, 203, 126),
      Glyph::BurnedBorder(Direction::Up) => (194, 128, 203, 130),
      Glyph::BurnedBorder(Direction::Down) => (194, 132, 203, 134),

      Glyph::Monster(kind, dir, digging, anim) => {
        let anim = anim as u8;
        let (pos_x, pos_y) = match kind {
          ActorKind::Furry => (160, 50),
          ActorKind::Grenadier => (160, 60),
          ActorKind::Slime => (160, 70),
          ActorKind::Alien => (0, 80),
          ActorKind::Player1 if digging == Digging::Pickaxe => (160, 200),
          ActorKind::Player2 if digging == Digging::Pickaxe => (0, 200),
          ActorKind::Player3 if digging == Digging::Pickaxe => (0, 210),
          ActorKind::Player4 if digging == Digging::Pickaxe => (160, 210),
          ActorKind::Player1 => (160, 10),
          ActorKind::Player2 => (160, 0),
          ActorKind::Player3 => (160, 30),
          ActorKind::Player4 => (160, 40),
        };
        let pos_x = pos_x + (dir as i16) * 40 + i16::from(anim) * 10;
        (pos_x, pos_y, pos_x + 9, pos_y + 9)
      }
    };
    Rect::new(
      i32::from(left),
      i32::from(top),
      (right - left + 1) as u32,
      (bottom - top + 1) as u32,
    )
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

/// Table for mapping equipment type to texture coordinates. Note that this list must be consistent
/// with the `Equipment` enum.
const EQUIPMENT_GLYPHS: [(i16, i16); Equipment::TOTAL] = [
  (0, 170),
  (30, 170),
  (60, 170),
  (216, 140),
  (240, 170),
  (210, 170),
  (246, 140),
  (270, 170),
  (90, 170),
  (120, 170),
  (246, 110),
  (90, 140),
  (150, 170),
  (180, 170),
  (276, 140),
  (276, 110),
  (216, 110),
  (0, 140),
  (30, 140),
  (60, 140),
  (30, 40),
  (232, 80),
  (262, 80),
  (0, 40),
  (105, 40),
  (60, 40),
  (0, 90),
];

/// FIXME: we perhaps can map monsters, too, even though we actually never render them as map cells
///  this could be useful for editor later

/// Map unmapped to "item 182" image.
const UNMAPPED: (i16, i16) = (50, 70);
/// Note: this mapping is offset by 0x30 and ends at 0xB6
const MAP_GLYPHS: [(i16, i16); 135] = [
  (0, 0),
  (10, 0),
  (20, 0),
  (30, 0),
  (40, 0),
  (50, 0),
  (60, 0),
  (70, 0),
  (80, 0),
  (90, 0),
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  (100, 0),
  (110, 0),
  (120, 0),
  (130, 0),
  (140, 0),
  (150, 0),
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  (0, 10),
  (10, 10),
  (20, 10),
  (90, 10),
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  (100, 10),
  (110, 10),
  (120, 10),
  (130, 10),
  (140, 10),
  (150, 10),
  (20, 30),
  (30, 30),
  (40, 30),
  (50, 30),
  (0, 30),
  (10, 30),
  (60, 30),
  UNMAPPED,
  (70, 30),
  (100, 30),
  (90, 30),
  UNMAPPED,
  (150, 30),
  UNMAPPED,
  UNMAPPED,
  UNMAPPED,
  (0, 20),
  (10, 20),
  (20, 20),
  UNMAPPED,
  UNMAPPED,
  (110, 30),
  (120, 30),
  (130, 30),
  (40, 10),
  (50, 10),
  (60, 10),
  (70, 10),
  (80, 10),
  (90, 10),
  (90, 10),
  (100, 10),
  (110, 10),
  UNMAPPED,
  (90, 10),
  (50, 20),
  (60, 20),
  (70, 20),
  (80, 20),
  (90, 20),
  (100, 20),
  (110, 20),
  (120, 20),
  (130, 20),
  (140, 20),
  (150, 20),
  (160, 20),
  (170, 20),
  (180, 20),
  (190, 20),
  (200, 20),
  (210, 20),
  (220, 20),
  (230, 20),
  (240, 20),
  (250, 20),
  (260, 20),
  (270, 20),
  (280, 20),
  (290, 20),
  (40, 20),
  (300, 20),
  (310, 20),
  (310, 20),
  (310, 20),
  (310, 20),
  UNMAPPED,
  (140, 30),
  (150, 40),
  (0, 70),
  (10, 70),
  (20, 70),
  (140, 40),
  (90, 10),
  (100, 10),
  (110, 10),
  (136, 50),
  (30, 70),
  (40, 70),
  (50, 70),
];
