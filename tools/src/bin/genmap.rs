use std::path::{Path, PathBuf};
use thiserror::Error;

/// Generate random map file
#[derive(structopt::StructOpt)]
struct Args {
  /// Map file to save result
  #[structopt(parse(from_os_str))]
  output: PathBuf,
  #[structopt(long, short, default_value = "75")]
  treasures: u8,
}

#[derive(Debug, Error)]
enum ToolError {
  #[error("Failed to write map to '{path}'")]
  OutputWriteError {
    path: PathBuf,
    #[source]
    source: anyhow::Error,
  },
}

/// Convert spy file into PNG
fn main() -> Result<(), anyhow::Error> {
  let args: Args = structopt::StructOpt::from_args();

  let map = mb_reloaded::world::map::LevelMap::random_map(args.treasures);
  let data = map.to_file_map();

  write_map(&args.output, &data).map_err(|source| ToolError::OutputWriteError {
    path: args.output,
    source,
  })?;
  Ok(())
}

fn write_map(path: &Path, map: &[u8]) -> Result<(), anyhow::Error> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }

  std::fs::write(path, map)?;
  Ok(())
}
