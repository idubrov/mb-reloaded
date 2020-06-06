use crate::error::ApplicationError::SdlError;
use sdl2::audio::{AudioCVT, AudioFormat};
use sdl2::mixer::Chunk;

/// VOC files seems to be unsigned, eight bits, 1 channel, 9600 Hz.
#[allow(unused)]
pub fn from_voc_bytes(data: Vec<u8>, rate: Option<i32>) -> Result<Chunk, anyhow::Error> {
  let (frequency, format, channels) = sdl2::mixer::query_spec().map_err(SdlError)?;

  // Need to convert audio format between sdl2::mixer and sdl2::audio
  let format = match format {
    sdl2::mixer::AUDIO_U8 => AudioFormat::U8,
    sdl2::mixer::AUDIO_S8 => AudioFormat::S8,
    sdl2::mixer::AUDIO_U16LSB => AudioFormat::U16LSB,
    sdl2::mixer::AUDIO_S16LSB => AudioFormat::S16LSB,
    sdl2::mixer::AUDIO_U16MSB => AudioFormat::U16MSB,
    sdl2::mixer::AUDIO_S16MSB => AudioFormat::S16MSB,
    sdl2::mixer::AUDIO_S32LSB => AudioFormat::S32LSB,
    sdl2::mixer::AUDIO_S32MSB => AudioFormat::S32MSB,
    sdl2::mixer::AUDIO_F32LSB => AudioFormat::F32LSB,
    sdl2::mixer::AUDIO_F32MSB => AudioFormat::F32MSB,
    _other => unreachable!(),
  };
  let converter = AudioCVT::new(
    AudioFormat::U8,
    1,
    rate.unwrap_or(9600),
    format,
    channels as u8,
    frequency,
  )
  .map_err(SdlError)?;
  let data = converter.convert(data);
  let chunk = Chunk::from_raw_buffer(data.into()).map_err(SdlError)?;
  Ok(chunk)
}
