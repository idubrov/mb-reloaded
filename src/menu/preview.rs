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
  if (value >= MapValue::Map37 && value <= MapValue::Map39)
    || (value >= MapValue::Map41 && value <= MapValue::Map46)
    || value == MapValue::MapA4
    || value == MapValue::Map70
    || value == MapValue::Map71
  {
    9
  } else if value == MapValue::Map73 || (value >= MapValue::Map92 && value <= MapValue::Map9A) {
    5
  } else if value == MapValue::Passage
    || value == MapValue::Map66
    || value == MapValue::MapAF
    || value == MapValue::Map65
  {
    14
  } else if value == MapValue::MetalWall {
    8
  } else if value == MapValue::BioMass {
    4
  } else {
    12
  }
}
