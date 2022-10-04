use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use thiserror::Error;
use clap::Parser;

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
#[derive(Parser)]
struct Args {
  /// FONT file to load
  #[arg(long, short, value_name = "FILE")]
  input: PathBuf,

  /// PNG file to save result
  #[arg(long, short, value_name = "FILE")]
  output: PathBuf,
}

/// Convert font file into PNG
fn main() -> Result<(), anyhow::Error> {
  let args: Args = Args::parse();

  let data = std::fs::read(&args.input).map_err(|source| ToolError::InputReadError {
    path: args.input.to_owned(),
    source: source.into(),
  })?;
  let image = mb_reloaded::fonts::decode_font(&data).map_err(|_| ToolError::InvalidFontFile {
    path: args.input.to_owned(),
  })?;

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
  encoder.set_color(png::ColorType::Rgba);
  encoder.set_depth(png::BitDepth::Eight);
  let mut writer = encoder.write_header()?;
  writer.write_image_data(image)?;
  Ok(())
}
