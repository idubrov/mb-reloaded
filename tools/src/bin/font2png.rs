use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
enum ToolError {
    #[error("Failed to load an input SPY file from '{path}'")]
    InputReadError {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },
    #[error("Failed to write an output PNG to '{path}'")]
    OutputWriteError {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },
    #[error("Font file '{path}' is in invalid format")]
    InvalidFontFile { path: PathBuf },
}

/// Convert FON file (font) into a PNG image
#[derive(structopt::StructOpt)]
struct Args {
    /// FONT file to load
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// PNG file to save result
    #[structopt(parse(from_os_str))]
    output: PathBuf,
}

/// Convert font file into PNG
fn main() -> Result<(), anyhow::Error> {
    let args: Args = structopt::StructOpt::from_args();

    let data = std::fs::read(&args.input).map_err(|source| ToolError::InputReadError {
        path: args.input.to_owned(),
        source: source.into(),
    })?;

    if data.len() != 256 * 8 {
        return Err(ToolError::InvalidFontFile { path: args.input }.into());
    }

    // Translate glyphs into PNG matrix 16x16
    let mut image = Vec::with_capacity(256 * 8 * 8);
    for glyph_row in 0..16 {
        for line in 0..8 {
            for glyph_col in 0..16 {
                image.push(data[(glyph_row * 16 + glyph_col) * 8 + line]);
            }
        }
    }

    write_image(&args.output, &image).map_err(|source| ToolError::OutputWriteError {
        path: args.output.to_owned(),
        source,
    })?;
    Ok(())
}

fn write_image(path: &Path, image: &[u8]) -> Result<(), anyhow::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(path)?;
    let buf = BufWriter::new(file);
    let mut encoder = png::Encoder::new(buf, 16 * 8, 16 * 8);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::One);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image)?;
    Ok(())
}
