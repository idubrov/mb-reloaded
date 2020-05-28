//! Tools to work with SPY files
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// SDL texture created from a SPY file with palette.
pub struct TexturePalette<'t> {
  pub texture: Texture<'t>,
  /// We only load 16 first colors as other part of palette is not used
  pub palette: [Color; 16],
}

#[derive(Debug, Error)]
#[error("Provided SPY file is not in a valid SPY file format")]
pub struct InvalidSpyFile;

#[derive(Debug, Error)]
#[error("Provided PPM file is not in a valid PPM file format")]
pub struct InvalidPpmFile;

/// Raw data for the decoded image.
pub struct DecodedImage {
  pub width: u32,
  pub height: u32,
  pub palette: [Color; 16],
  /// Image bytes, 3 bytes per pixel, RGB.
  pub image: Vec<u8>,
}

#[derive(Debug, Error)]
#[error("Failed to load texture from '{path}'")]
pub struct TextureLoadingFailed {
  path: PathBuf,
  source: anyhow::Error,
}

/// Format of the texture to load
pub enum TextureFormat {
  /// Whole screen images, with palette and simple RLE encoding. Encode 4 independent bitplanes.
  SPY,
  /// Partial screen images with simple RLE encoding. Encode each color directly.
  PPM,
}

/// Load texture at the given path
pub fn load_texture<'t>(
  texture_creator: &'t TextureCreator<WindowContext>,
  path: &Path,
  format: TextureFormat,
) -> Result<TexturePalette<'t>, TextureLoadingFailed> {
  load_texture_internal(texture_creator, path, format).map_err(|source| TextureLoadingFailed {
    path: path.to_owned(),
    source,
  })
}

fn load_texture_internal<'t>(
  texture_creator: &'t TextureCreator<WindowContext>,
  path: &Path,
  format: TextureFormat,
) -> Result<TexturePalette<'t>, anyhow::Error> {
  let data = std::fs::read(path)?;

  let decoded = match format {
    TextureFormat::SPY => decode_spy(SCREEN_WIDTH, SCREEN_HEIGHT, &data)?,
    TextureFormat::PPM => decode_ppm(&data)?,
  };
  let mut texture =
    texture_creator.create_texture_static(PixelFormatEnum::RGB24, decoded.width as u32, decoded.height as u32)?;

  texture.update(None, &decoded.image, (decoded.width as usize) * 3)?;
  Ok(TexturePalette {
    palette: decoded.palette,
    texture,
  })
}

/// Decode SPY file into an RGB image. Image is returned as raw bytes, with 3 bytes per color (red,
/// green and blue).
///
/// SPY is a simple format with 768 bytes of palette coming first, then four bitplanes encoded
/// with run-length encoding. Each bitplane is one bit of image pixel color (4 bitplanes means each
/// color is 4-bits). Colors are indices into the palette (only first 16 colors of the palette are
/// used).
pub fn decode_spy(width: u32, height: u32, data: &[u8]) -> Result<DecodedImage, InvalidSpyFile> {
  // Each bit of a bitplane is a pixel in the output image.
  let bitplane_len = (width as usize) * (height as usize) / 8;

  // Header should include palette
  if data.len() < 768 {
    return Err(InvalidSpyFile);
  }

  let (palette, data) = data.split_at(768);
  let mut it = data.iter().copied();

  // Planes go one after another, each plane is decoded until we get width * height / 8 bytes.
  let plane0 = decode_plane(bitplane_len, &mut it)?;
  let plane1 = decode_plane(bitplane_len, &mut it)?;
  let plane2 = decode_plane(bitplane_len, &mut it)?;
  let plane3 = decode_plane(bitplane_len, &mut it)?;

  // Each plane is 8 bits, we have 4 planes and images have 16 colors (4 bits)
  // We expand into 3 RGB components.
  // bitplane_len * 8 (bits per plane) * 4 (planes) / 4 (bits) * 3 (components)
  let mut image = Vec::with_capacity(bitplane_len * 24);
  for idx in 0..bitplane_len {
    for bit in (0..8).rev() {
      let bit0 = (plane0[idx] >> bit) & 1;
      let bit1 = ((plane1[idx] >> bit) & 1) << 1;
      let bit2 = ((plane2[idx] >> bit) & 1) << 2;
      let bit3 = ((plane3[idx] >> bit) & 1) << 3;
      let color = (bit0 | bit1 | bit2 | bit3) as usize;

      image.push(palette[color * 3]);
      image.push(palette[color * 3 + 1]);
      image.push(palette[color * 3 + 2]);
    }
  }
  Ok(DecodedImage {
    width: SCREEN_WIDTH,
    height: SCREEN_HEIGHT,
    palette: decode_palette(palette),
    image,
  })
}

/// Simple run-length encoding. `1` is interpreted as a run-length instruction. Everything else
/// is placed directly into the output.
fn decode_plane(bitplane_len: usize, mut it: impl Iterator<Item = u8>) -> Result<Vec<u8>, InvalidSpyFile> {
  let mut image = Vec::new();
  while image.len() < bitplane_len {
    let val = it.next().ok_or(InvalidSpyFile)?;
    if val != 1 {
      image.push(val);
    } else {
      // Run-length encoding
      let val = it.next().ok_or(InvalidSpyFile)?;
      let len = it.next().ok_or(InvalidSpyFile)?;
      for _ in 0..len {
        image.push(val);
      }
    }
  }
  Ok(image)
}

pub fn decode_ppm(data: &[u8]) -> Result<DecodedImage, InvalidPpmFile> {
  // Header, palette and palette flag.
  if data.len() < 128 + 768 + 1 {
    return Err(InvalidPpmFile);
  }
  let from_y = u32::from(data[6]) + (u32::from(data[7]) << 8);
  let to_y = u32::from(data[10]) + (u32::from(data[11]) << 8);
  let width = u32::from(data[0x42]) + (u32::from(data[0x43]) << 8);
  let height = to_y - from_y;
  let mut it = data[128..data.len() - 769].iter().copied();
  let palette = &data[data.len() - 768..];

  let mut image = Vec::with_capacity((width as usize) * (height as usize) * 3);
  for _ in 0..height {
    let mut x = 0;
    while x < width {
      let value = it.next().ok_or(InvalidPpmFile)?;
      if (value & 0xC0) == 0xC0 {
        let len = u32::from(value) & 0x3F;
        let color = usize::from(it.next().ok_or(InvalidPpmFile)?);

        if x + len > width {
          return Err(InvalidPpmFile);
        }
        for _ in 0..len {
          image.push(palette[color * 3]);
          image.push(palette[color * 3 + 1]);
          image.push(palette[color * 3 + 2]);
        }
        x += len;
      } else {
        let color = usize::from(value);
        image.push(palette[color * 3]);
        image.push(palette[color * 3 + 1]);
        image.push(palette[color * 3 + 2]);
        x += 1;
      }
    }
  }

  debug_assert_eq!(image.len(), (width as usize) * (height as usize) * 3);
  Ok(DecodedImage {
    width,
    height,
    palette: decode_palette(palette),
    image,
  })
}

/// Decode palette from the image; note that we only read the first 16 colors.
fn decode_palette(data: &[u8]) -> [Color; 16] {
  let mut palette: [Color; 16] = [Color::BLACK; 16];
  for color in 0..16 {
    let r = data[color * 3];
    let g = data[color * 3 + 1];
    let b = data[color * 3 + 2];
    palette[color] = Color::RGB(r, g, b);
  }
  palette
}
