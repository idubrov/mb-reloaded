use crate::error::ApplicationError::SdlError;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Provided FON file is not in a valid FON file format")]
pub struct InvalidFontFile;

#[derive(Debug, Error)]
#[error("Failed to load font from '{path}'")]
pub struct FontLoadingFailed {
    path: PathBuf,
    source: anyhow::Error,
}

pub struct Font {
    texture: RefCell<Texture>,
}

impl Font {
    pub fn render(
        &self,
        canvas: &mut WindowCanvas,
        x: i32,
        y: i32,
        color: Color,
        text: &str,
    ) -> Result<(), anyhow::Error> {
        let mut texture = self.texture.borrow_mut();
        texture.set_color_mod(color.r, color.g, color.b);

        let mut source = Rect::new(0, 0, 8, 8);
        let mut target = Rect::new(x, y, 8, 8);
        for ch in text.chars() {
            let ch: u8 = if ch.is_ascii() { ch as u8 } else { b' ' };
            source.set_x(((ch % 16) as i32) * 8);
            source.set_y(((ch / 16) as i32) * 8);
            canvas.copy(&texture, source, target).map_err(SdlError)?;
            target.set_x(target.x() + 8);
        }
        Ok(())
    }
}

/// Load font texture
pub fn load_font(
    texture_creator: &TextureCreator<WindowContext>,
    path: &Path,
) -> Result<Font, FontLoadingFailed> {
    let texture =
        load_font_internal(texture_creator, path).map_err(|source| FontLoadingFailed {
            path: path.to_owned(),
            source,
        })?;
    Ok(Font {
        texture: RefCell::new(texture),
    })
}

fn load_font_internal(
    texture_creator: &TextureCreator<WindowContext>,
    path: &Path,
) -> Result<Texture, anyhow::Error> {
    let data = std::fs::read(path)?;
    let data = decode_font(&data)?;
    let mut texture =
        texture_creator.create_texture_static(PixelFormatEnum::RGBA32, 16 * 8, 16 * 8)?;
    texture.update(None, &data, 16 * 8 * 4)?;
    texture.set_blend_mode(BlendMode::Blend);
    Ok(texture)
}

/// Decode font into image bytes, with size of 128 by 128 pixels (each character is 8 pixels wide and
/// 8 pixels high). Each pixel is four bytes (R, G, B and A components). Glyphs are placed in a matrix
/// of 16 characters by 16 characters (left to right, top to bottom).
pub fn decode_font(data: &[u8]) -> Result<Vec<u8>, InvalidFontFile> {
    if data.len() != 256 * 8 {
        return Err(InvalidFontFile);
    }

    // Translate glyphs into PNG matrix 2048x8
    let mut image = Vec::with_capacity(256 * 8 * 8 * 4);
    for row in 0..16 {
        for glyph_line in 0..8 {
            for col in 0..16 {
                for bit in 0..8 {
                    let mask = 1 << (7 - bit);
                    let value = data[(row * 16 + col) * 8 + glyph_line];
                    if (value & mask) != 0 {
                        image.push(255);
                        image.push(255);
                        image.push(255);
                        image.push(255);
                    } else {
                        image.push(0);
                        image.push(0);
                        image.push(0);
                        image.push(0);
                    }
                }
            }
        }
    }
    Ok(image)
}
