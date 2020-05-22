//! Tools to work with SPY files
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Provided SPY file is not in a valid SPY file format")]
pub struct InvalidSpyFile;

#[derive(Debug, Error)]
#[error("Failed to load texture from '{path}'")]
pub struct TextureLoadingFailed {
    path: PathBuf,
    source: anyhow::Error,
}

/// Load texture at the given path
pub fn load_texture(
    texture_creator: &TextureCreator<WindowContext>,
    path: &Path,
) -> Result<Texture, TextureLoadingFailed> {
    load_texture_internal(texture_creator, path).map_err(|source| TextureLoadingFailed {
        path: path.to_owned(),
        source,
    })
}

fn load_texture_internal(
    texture_creator: &TextureCreator<WindowContext>,
    path: &Path,
) -> Result<Texture, anyhow::Error> {
    let spy_data = std::fs::read(path)?;
    let image = crate::spy::decode_spy(SCREEN_WIDTH, SCREEN_HEIGHT, &spy_data)?;
    let mut texture = texture_creator.create_texture_static(
        PixelFormatEnum::RGB24,
        SCREEN_WIDTH as u32,
        SCREEN_HEIGHT as u32,
    )?;
    texture.update(None, &image, SCREEN_WIDTH * 3)?;
    Ok(texture)
}

/// Decode SPY file into an RGB image. Image is returned as raw bytes, with 3 bytes per color (red,
/// green and blue).
///
/// SPY is a simple format with 768 bytes of palette coming first, then four bitplanes encoded
/// with run-length encoding. Each bitplane is one bit of image pixel color (4 bitplanes means each
/// color is 4-bits). Colors are indices into the palette (only first 16 colors of the palette are
/// used).
pub fn decode_spy(width: usize, height: usize, data: &[u8]) -> Result<Vec<u8>, InvalidSpyFile> {
    // Each bit of a bitplane is a pixel in the output image.
    let bitplane_len = width * height / 8;

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
    Ok(image)
}

/// Simple run-length encoding. `1` is interpreted as a run-length instruction. Everything else
/// is placed directly into the output.
fn decode_plane(
    bitplane_len: usize,
    mut it: impl Iterator<Item = u8>,
) -> Result<Vec<u8>, InvalidSpyFile> {
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
