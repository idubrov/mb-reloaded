//! Crate with lower-level functions to emulate sound effects of the original game.
//! `SDL_RegisterEffect` is not supported by Rust bindings in `sdl2` crate, so we use lower-level C API.
//! Due to `unsafe` use, this is extracted into a separate crate to keep main crate clean of unsafe.

use libc::{c_int, c_void};
use sdl2::audio::AudioFormatNum;
use sdl2::mixer::{Channel, Chunk};
use std::sync::Arc;

const BUF_LEN: usize = 4096;
static mut BUF: [u8; BUF_LEN] = [0; BUF_LEN];

/// Used to make mixer to play something. We don't really use these values at all -- we generate
/// sound samples directly in the registered effect.
static mut PLACEHOLDER: sdl2_sys::mixer::Mix_Chunk = sdl2_sys::mixer::Mix_Chunk {
  allocated: 0,
  abuf: unsafe { &mut BUF as *mut [u8] as *mut u8 },
  alen: BUF_LEN as u32,
  volume: 128,
};

/// Play sound effect on a given channel with a given playback frequency located at `position`.
pub fn play_sound_sample(channel: Channel, frequency: i32, chunk: Arc<[u8]>, position: f32) -> Result<(), String> {
  let placeholder = Chunk {
    raw: unsafe { &mut PLACEHOLDER as *mut _ },
    owned: false,
  };
  let channel = channel.play(&placeholder, -1)?;
  let (mixer_frequency, format, channels) = sdl2::mixer::query_spec()?;
  let effect = Box::new(SampleCallback {
    channels: channels as usize,
    chunk,
    play_frequency: frequency,
    mixer_frequency,
    target_sample_offset: 0,
    position,
  });
  let user_ptr = Box::into_raw(effect);

  let Channel(chan) = channel;
  let ret = unsafe {
    sdl2_sys::mixer::Mix_RegisterEffect(
      chan,
      gen_pitch_callback(format),
      Some(pitch_done_cb),
      user_ptr as *mut _,
    )
  };
  if ret == -1 {
    // Need to free the memory
    unsafe {
      let _ = Box::from_raw(user_ptr);
    }
    Err(sdl2::get_error())
  } else {
    Ok(())
  }
}

fn gen_pitch_callback(format: sdl2::mixer::AudioFormat) -> sdl2_sys::mixer::Mix_EffectFunc_t {
  let func = match format {
    sdl2::mixer::AUDIO_U8 => pitch_effect_cb_template::<u8>,
    sdl2::mixer::AUDIO_S8 => pitch_effect_cb_template::<i8>,
    sdl2::mixer::AUDIO_U16LSB => pitch_effect_cb_template::<u16>,
    sdl2::mixer::AUDIO_S16LSB => pitch_effect_cb_template::<i16>,
    sdl2::mixer::AUDIO_S32LSB => pitch_effect_cb_template::<i32>,
    sdl2::mixer::AUDIO_F32LSB => pitch_effect_cb_template::<f32>,

    // Need some types that will do conversion from MSB=>LBS or what?
    sdl2::mixer::AUDIO_U16MSB | sdl2::mixer::AUDIO_S16MSB | sdl2::mixer::AUDIO_S32MSB | sdl2::mixer::AUDIO_F32MSB => {
      unimplemented!()
    }
    _other => unreachable!(),
  };
  Some(func)
}

struct SampleCallback {
  /// Sample we want to play (single channel, unsigned, 8-bit).
  chunk: Arc<[u8]>,
  /// Frequency we want to play the sample
  play_frequency: i32,
  /// Horizontal pozition: 0.0 is the leftmost, 1.0 is the rightmost
  position: f32,
  /// Amount of channels current mixer has
  channels: usize,
  /// Frequency of the mixer we are targeting
  mixer_frequency: i32,
  /// Sample index (in the output format; basically, amount of samples we have generated so far).
  target_sample_offset: usize,
}

impl SampleCallback {
  fn generate_samples<T: AudioFormatNum + IntoSample>(&mut self, _chan: c_int, stream: &mut [T]) -> bool {
    let samples = stream.len() / self.channels;
    for sample in 0..samples {
      let output = &mut stream[(sample * self.channels)..][..self.channels];

      let target_sample = self.target_sample_offset + sample;
      let source_pos = (target_sample as f32) * (self.play_frequency as f32) / (self.mixer_frequency as f32);
      // round to floor
      let index = source_pos as usize;

      // Have source samples to interpolate
      if index < self.chunk.len() {
        let first = self.chunk[index];
        let second = self.chunk.get(index + 1).copied().unwrap_or(first);

        let fract = source_pos.fract();
        let first = f32::from(first.wrapping_sub(u8::SILENCE) as i8) / 256.0;
        let second = f32::from(second.wrapping_sub(u8::SILENCE) as i8) / 256.0;
        let sample = first * fract + second * (1.0 - fract);
        // Clamp the output
        let sample = if sample < -0.5 {
          -0.5
        } else if sample > 0.5 {
          0.5
        } else {
          sample
        };
        if self.channels == 1 {
          output[0] = IntoSample::from_f32(sample);
        } else {
          output[0] = IntoSample::from_f32(sample * (1.0 - self.position));
          output[1] = IntoSample::from_f32(sample * self.position);
        }
      } else {
        // We are done playing! Fill the rest with the silence and return termination flag.
        for item in &mut stream[sample * self.channels..] {
          *item = T::SILENCE;
        }
        return true;
      }
    }
    self.target_sample_offset += samples;
    false
  }
}

extern "C" fn pitch_effect_cb_template<T: AudioFormatNum + IntoSample>(
  chan: c_int,
  stream: *mut c_void,
  len: c_int,
  udata: *mut c_void,
) {
  // Sanity check
  if udata.is_null() {
    return;
  }

  let len = len as usize;
  let stream = unsafe { std::slice::from_raw_parts_mut(stream as *mut T, len / std::mem::size_of::<T>()) };

  let halt = {
    // Need to make sure we don't have mutable reference borrow after this block: pointer might get
    // deallocated when we call `.halt()`.
    let effect = unsafe { &mut *(udata as *mut SampleCallback) };
    effect.generate_samples(chan, stream)
  };

  if halt {
    // `udata` be de-allocated after this point! Not safe to use.
    Channel(chan).halt();
  }
}

extern "C" fn pitch_done_cb(_chan: c_int, udata: *mut c_void) {
  // Sanity check
  if udata.is_null() {
    return;
  }
  let udata: *mut SampleCallback = udata as *mut _;
  unsafe {
    // Drop so we free all the memory we have allocated
    let _ = Box::from_raw(udata);
  }
}

/// Convert floating point in the range of the (-1.0f, 1.0f) to target sample type. 0.0f is the silence.
pub(crate) trait IntoSample: Copy {
  fn from_f32(sample: f32) -> Self;
}

impl IntoSample for u8 {
  fn from_f32(sample: f32) -> Self {
    (i8::from_f32(sample) as u8).wrapping_add(u8::SILENCE)
  }
}

impl IntoSample for i8 {
  fn from_f32(sample: f32) -> Self {
    (sample * 8.0_f32.exp2()) as i8
  }
}

impl IntoSample for u16 {
  fn from_f32(sample: f32) -> Self {
    (i16::from_f32(sample) as u16).wrapping_add(u16::SILENCE)
  }
}

impl IntoSample for i16 {
  fn from_f32(sample: f32) -> Self {
    (sample * 16.0_f32.exp2()) as i16
  }
}

impl IntoSample for i32 {
  fn from_f32(sample: f32) -> Self {
    (sample * 32.0_f32.exp2()) as i32
  }
}

impl IntoSample for f32 {
  fn from_f32(sample: f32) -> Self {
    sample
  }
}
