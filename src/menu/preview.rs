use crate::map::{LevelMap, MapValue};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;

/// Generate texture for the map preview.
pub fn generate_preview<'t>(
  map: &LevelMap,
  texture_creator: &'t TextureCreator<WindowContext>,
  palette: &[Color; 16],
) -> Result<Texture<'t>, anyhow::Error> {
  let mut texture = texture_creator.create_texture_static(PixelFormatEnum::RGB24, 64, 45)?;
  let mut image = Vec::with_capacity(45 * 64 * 3);
  for row in 0..45 {
    for col in 0..64 {
      let color = preview_pixel(map[row][col]);
      let color = palette[color];
      image.push(color.r);
      image.push(color.g);
      image.push(color.b);
    }
  }
  texture.update(None, &image, 64 * 3)?;
  Ok(texture)
}

/// Get color index for preview pixel of a given map value.
fn preview_pixel(value: MapValue) -> usize {
  if (value >= MapValue::StoneTopLeft && value <= MapValue::StoneBottomRight)
    || (value >= MapValue::StoneBottomLeft && value <= MapValue::Stone4)
    || value == MapValue::Barrel
    || value == MapValue::StoneLightCracked
    || value == MapValue::StoneHeavyCracked
  {
    9
  } else if value == MapValue::Diamond || (value >= MapValue::GoldShield && value <= MapValue::GoldCrown) {
    5
  } else if value.is_passable() || value == MapValue::Mine {
    14
  } else if value == MapValue::MetalWall {
    8
  } else if value == MapValue::BioMass {
    4
  } else {
    12
  }
}
