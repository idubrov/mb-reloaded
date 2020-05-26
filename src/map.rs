use num_enum::TryFromPrimitive;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Invalid map format")]
pub struct InvalidMap;

pub struct MapData {
  data: Vec<MapValue>,
}

impl MapData {
  /// Create statically typed map from a vector of bytes.
  pub fn from_bytes(data: Vec<u8>) -> Result<MapData, InvalidMap> {
    // Each map is 45 lines 66 bytes each (64 columns plus "\r\n" at the end of each row)
    if data.len() != 2970 {
      return Err(InvalidMap);
    }
    Ok(MapData {
      // We could transmute here, but let's avoid all unsafe; amount of data is pretty small.
      data: data.into_iter().map(|v| MapValue::try_from(v).unwrap()).collect(),
    })
  }

  /// Generate image for the preview.
  pub fn generate_preview<'t>(
    &self,
    texture_creator: &'t TextureCreator<WindowContext>,
    palette: &[Color; 16],
  ) -> Result<Texture<'t>, anyhow::Error> {
    let mut texture = texture_creator.create_texture_static(PixelFormatEnum::RGB24, 64, 45)?;
    let mut image = Vec::with_capacity(45 * 64 * 3);
    for row in 0..45 {
      for col in 0..64 {
        // Two last bytes of the row are 0xd 0xa (newline), so 64 + 2 = 66
        let offset = row * 66 + col;
        let color = preview_pixel(self.data[offset]);
        let color = palette[color];
        image.push(color.r);
        image.push(color.g);
        image.push(color.b);
      }
    }
    texture.update(None, &image, 64 * 3)?;
    Ok(texture)
  }
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

/// Enum for all possible map values.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, TryFromPrimitive, PartialOrd)]
pub enum MapValue {
  Map00 = 0,
  Map01,
  Map02,
  Map03,
  Map04,
  Map05,
  Map06,
  Map07,
  Map08,
  Map09,
  Map0A,
  Map0B,
  Map0C,
  Map0D,
  Map0E,
  Map0F,
  Map10,
  Map11,
  Map12,
  Map13,
  Map14,
  Map15,
  Map16,
  Map17,
  Map18,
  Map19,
  Map1A,
  Map1B,
  Map1C,
  Map1D,
  Map1E,
  Map1F,
  Map20,
  Map21,
  Map22,
  Map23,
  Map24,
  Map25,
  Map26,
  Map27,
  Map28,
  Map29,
  Map2A,
  Map2B,
  Map2C,
  Map2D,
  Map2E,
  Map2F,
  Passage,
  MetalWall,
  Map32,
  Map33,
  Map34,
  Map35,
  Map36,
  Map37,
  Map38,
  Map39,
  Map3A,
  Map3B,
  Map3C,
  Map3D,
  Map3E,
  Map3F,
  Map40,
  Map41,
  Map42,
  Map43,
  Map44,
  Map45,
  Map46,
  Map47,
  Map48,
  Map49,
  Map4A,
  Map4B,
  Map4C,
  Map4D,
  Map4E,
  Map4F,
  Map50,
  Map51,
  Map52,
  Map53,
  Map54,
  Map55,
  Map56,
  Map57,
  Map58,
  Map59,
  Map5A,
  Map5B,
  Map5C,
  Map5D,
  Map5E,
  Map5F,
  Map60,
  Map61,
  Map62,
  Map63,
  Map64,
  Map65,
  Map66,
  Map67,
  Map68,
  Map69,
  Map6A,
  Map6B,
  Map6C,
  Map6D,
  Map6E,
  BioMass,
  Map70,
  Map71,
  Map72,
  Map73,
  Map74,
  Map75,
  Map76,
  Map77,
  Map78,
  Map79,
  Map7A,
  Map7B,
  Map7C,
  Map7D,
  Map7E,
  Map7F,
  Map80,
  Map81,
  Map82,
  Map83,
  Map84,
  Map85,
  Map86,
  Map87,
  Map88,
  Map89,
  Map8A,
  Map8B,
  Map8C,
  Map8D,
  Map8E,
  Map8F,
  Map90,
  Map91,
  Map92,
  Map93,
  Map94,
  Map95,
  Map96,
  Map97,
  Map98,
  Map99,
  Map9A,
  Map9B,
  Map9C,
  Map9D,
  Map9E,
  Map9F,
  MapA0,
  MapA1,
  MapA2,
  MapA3,
  MapA4,
  MapA5,
  MapA6,
  MapA7,
  MapA8,
  MapA9,
  MapAA,
  MapAB,
  MapAC,
  MapAD,
  MapAE,
  MapAF,
  MapB0,
  MapB1,
  MapB2,
  MapB3,
  MapB4,
  MapB5,
  MapB6,
  MapB7,
  MapB8,
  MapB9,
  MapBA,
  MapBB,
  MapBC,
  MapBD,
  MapBE,
  MapBF,
  MapC0,
  MapC1,
  MapC2,
  MapC3,
  MapC4,
  MapC5,
  MapC6,
  MapC7,
  MapC8,
  MapC9,
  MapCA,
  MapCB,
  MapCC,
  MapCD,
  MapCE,
  MapCF,
  MapD0,
  MapD1,
  MapD2,
  MapD3,
  MapD4,
  MapD5,
  MapD6,
  MapD7,
  MapD8,
  MapD9,
  MapDA,
  MapDB,
  MapDC,
  MapDD,
  MapDE,
  MapDF,
  MapE0,
  MapE1,
  MapE2,
  MapE3,
  MapE4,
  MapE5,
  MapE6,
  MapE7,
  MapE8,
  MapE9,
  MapEA,
  MapEB,
  MapEC,
  MapED,
  MapEE,
  MapEF,
  MapF0,
  MapF1,
  MapF2,
  MapF3,
  MapF4,
  MapF5,
  MapF6,
  MapF7,
  MapF8,
  MapF9,
  MapFA,
  MapFB,
  MapFC,
  MapFD,
  MapFE,
  MapFF,
}
