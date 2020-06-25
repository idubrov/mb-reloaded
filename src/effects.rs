use crate::error::ApplicationError::SdlError;
use crate::world::map::MAP_COLS;
use crate::world::position::Cursor;
use sdl2::mixer::Channel;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Failed to load sound sample from '{path}'")]
pub struct SampleLoadingFailed {
  path: PathBuf,
  source: anyhow::Error,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoundEffect {
  Kili,
  Picaxe,
  Explos3,
}

/// VOC files are unsigned, eight bits, 1 channel, frequency defined at the playback time (typically 11000).
/// We use `Arc` here so we can give references to these samples to sound effects without worrying
/// about ownership.
struct RawSample(Arc<[u8]>);

pub struct SoundEffects {
  kili: RawSample,
  picaxe: RawSample,
  explos3: RawSample,
}

impl SoundEffects {
  /// Initialize game sound effects given the game directory
  pub fn new(path: &Path) -> Result<Self, anyhow::Error> {
    Ok(SoundEffects {
      kili: load_sample(path.join("KILI.VOC"))?,
      picaxe: load_sample(path.join("PICAXE.VOC"))?,
      explos3: load_sample(path.join("EXPLOS3.VOC"))?,
    })
  }

  /// Play sound effec
  pub fn play(&self, effect: SoundEffect, frequency: i32, location: Cursor) -> Result<(), anyhow::Error> {
    let position = f32::from(location.col) / f32::from(MAP_COLS - 1);
    let effect = match effect {
      SoundEffect::Kili => self.kili.0.clone(),
      SoundEffect::Picaxe => self.picaxe.0.clone(),
      SoundEffect::Explos3 => self.explos3.0.clone(),
    };
    // FIXME: reuse channels if all cannels are busy
    let channel = Channel::all();
    mb_sdl2_effects::play_sound_sample(channel, frequency, effect, position).map_err(SdlError)?;
    Ok(())
  }
}

fn load_sample(path: PathBuf) -> Result<RawSample, SampleLoadingFailed> {
  let data = std::fs::read(&path).map_err(|source| SampleLoadingFailed {
    path,
    source: source.into(),
  })?;
  Ok(RawSample(data.into()))
}
