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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEffect {
  Kili,
  Picaxe,
  Explos1,
  Explos2,
  Explos3,
  Explos4,
  Explos5,
  Aargh,
  Karjaisu,
  Pikkupom,
  Urethan,
  Applause,
}

/// VOC files are unsigned, eight bits, 1 channel, frequency defined at the playback time (typically 11000).
/// We use `Arc` here so we can give references to these samples to sound effects without worrying
/// about ownership.
#[derive(Clone)]
struct RawSample(Arc<[u8]>);

pub struct SoundEffects {
  kili: RawSample,
  picaxe: RawSample,
  explos1: RawSample,
  explos2: RawSample,
  explos3: RawSample,
  explos4: RawSample,
  explos5: RawSample,
  aargh: RawSample,
  karjaisu: RawSample,
  pikkupom: RawSample,
  urethan: RawSample,
  applause: RawSample,
}

impl SoundEffects {
  /// Initialize game sound effects given the game directory
  pub fn new(path: &Path) -> Result<Self, anyhow::Error> {
    Ok(SoundEffects {
      kili: load_sample(path.join("KILI.VOC"))?,
      picaxe: load_sample(path.join("PICAXE.VOC"))?,
      explos1: load_sample(path.join("EXPLOS1.VOC"))?,
      explos2: load_sample(path.join("EXPLOS2.VOC"))?,
      explos3: load_sample(path.join("EXPLOS3.VOC"))?,
      explos4: load_sample(path.join("EXPLOS4.VOC"))?,
      explos5: load_sample(path.join("EXPLOS5.VOC"))?,
      aargh: load_sample(path.join("AARGH.VOC"))?,
      karjaisu: load_sample(path.join("KARJAISU.VOC"))?,
      pikkupom: load_sample(path.join("PIKKUPOM.VOC"))?,
      urethan: load_sample(path.join("URETHAN.VOC"))?,
      applause: load_sample(path.join("APPLAUSE.VOC"))?,
    })
  }

  /// Play sound effec
  pub fn play(&self, effect: SoundEffect, frequency: i32, location: Cursor) -> Result<(), anyhow::Error> {
    let position = f32::from(location.col) / f32::from(MAP_COLS - 1);
    let effect = match effect {
      SoundEffect::Kili => &self.kili,
      SoundEffect::Picaxe => &self.picaxe,
      SoundEffect::Explos1 => &self.explos1,
      SoundEffect::Explos2 => &self.explos2,
      SoundEffect::Explos3 => &self.explos3,
      SoundEffect::Explos4 => &self.explos4,
      SoundEffect::Explos5 => &self.explos5,
      SoundEffect::Aargh => &self.aargh,
      SoundEffect::Karjaisu => &self.karjaisu,
      SoundEffect::Pikkupom => &self.pikkupom,
      SoundEffect::Urethan => &self.urethan,
      SoundEffect::Applause => &self.applause,
    };
    // FIXME: reuse channels if all cannels are busy
    let channel = Channel::all();
    mb_sdl2_effects::play_sound_sample(channel, frequency, effect.0.clone(), position).map_err(SdlError)?;
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
