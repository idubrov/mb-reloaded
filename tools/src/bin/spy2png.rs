use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Convert SPY file into a PNG image
#[derive(structopt::StructOpt)]
struct Args {
  /// SPY file to load
  #[structopt(parse(from_os_str))]
  input: PathBuf,

  /// PNG file to save result
  #[structopt(parse(from_os_str))]
  output: PathBuf,
}

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
}

// SPY files have fixed 640x480 size

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

/// Convert spy file into PNG
fn main() -> Result<(), anyhow::Error> {
  let args: Args = structopt::StructOpt::from_args();

  let data = std::fs::read(&args.input).map_err(|source| ToolError::InputReadError {
    path: args.input.to_owned(),
    source: source.into(),
  })?;

  let decoded = mb_reloaded::images::decode_spy(640, 480, &data)?;
  write_image(&args.output, &decoded.image).map_err(|source| ToolError::OutputWriteError {
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
  let mut encoder = png::Encoder::new(buf, WIDTH as u32, HEIGHT as u32);
  encoder.set_color(png::ColorType::RGB);
  encoder.set_depth(png::BitDepth::Eight);
  let mut writer = encoder.write_header()?;
  writer.write_image_data(&image)?;
  Ok(())
}
