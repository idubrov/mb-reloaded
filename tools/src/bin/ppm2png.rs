use clap::Parser;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Convert PPM file into a PNG image
#[derive(Parser)]
struct Args {
  /// PPM file to load
  #[arg(long, short, value_name = "FILE")]
  input: PathBuf,

  /// PNG file to save result
  #[arg(long, short, value_name = "FILE")]
  output: PathBuf,
}

#[derive(Debug, Error)]
enum ToolError {
  #[error("Failed to load an input PPM file from '{path}'")]
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
}

/// Convert PPM file into PNG
fn main() -> Result<(), anyhow::Error> {
  let args: Args = Args::parse();

  let data = std::fs::read(&args.input).map_err(|source| ToolError::InputReadError {
    path: args.input.to_owned(),
    source: source.into(),
  })?;

  let decoded = mb_reloaded::images::decode_ppm(&data)?;
  write_image(&args.output, decoded.width, decoded.height, &decoded.image).map_err(|source| {
    ToolError::OutputWriteError {
      path: args.output.to_owned(),
      source,
    }
  })?;

  Ok(())
}

fn write_image(path: &Path, width: u32, height: u32, image: &[u8]) -> Result<(), anyhow::Error> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }

  let file = File::create(path)?;
  let buf = BufWriter::new(file);
  let mut encoder = png::Encoder::new(buf, width, height);
  encoder.set_color(png::ColorType::Rgb);
  encoder.set_depth(png::BitDepth::Eight);
  let mut writer = encoder.write_header()?;
  writer.write_image_data(image)?;
  Ok(())
}
