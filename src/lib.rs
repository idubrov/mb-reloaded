use sdl2::event::Event;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{BlendMode, Texture, WindowCanvas};
use sdl2::EventPump;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

pub mod spy;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

#[derive(Debug, Error)]
enum ApplicationError {
    #[error("SDL error: {0}")]
    SdlError(String),
}

// For compactness of `.map_err()`
use ApplicationError::SdlError;

/// Data-compatible MineBombers 3.11 reimplementation
#[derive(structopt::StructOpt)]
struct Args {
    /// Path to the MineBombers 3.11 installation
    #[structopt(
        parse(from_os_str),
        default_value = ".",
        validator_os = is_valid_installation_directory,
    )]
    game_path: PathBuf,
}

struct MainApp {
    canvas: WindowCanvas,
    events: EventPump,
}

pub fn main() -> Result<(), anyhow::Error> {
    let args: Args = structopt::StructOpt::from_args();

    let sdl_context = sdl2::init().map_err(SdlError)?;
    let video = sdl_context.video().map_err(SdlError)?;
    let window = video
        .window("MineBombers Reloaded", WIDTH as u32, HEIGHT as u32)
        .position_centered()
        .allow_highdpi()
        .build()?;
    let canvas = window.into_canvas().build()?;
    let texture_creator = canvas.texture_creator();

    let title = std::fs::read(args.game_path.join("titlebe.spy"))?;
    let image = spy::decode_spy(WIDTH, HEIGHT, &title)?;
    let mut title = texture_creator.create_texture_static(
        PixelFormatEnum::RGB24,
        WIDTH as u32,
        HEIGHT as u32,
    )?;
    title.update(None, &image, WIDTH * 3)?;

    let mut main = MainApp {
        canvas,
        events: sdl_context.event_pump().map_err(SdlError)?,
    };

    // Capture the image for animation
    let mut animation = texture_creator.create_texture_target(
        PixelFormatEnum::RGB24,
        WIDTH as u32,
        HEIGHT as u32,
    )?;
    main.canvas.with_texture_canvas(&mut animation, |canvas| {
        canvas.copy(&title, None, None).unwrap();
    })?;

    main.animate(&mut animation, Animation::FadeUp, 7)?;

    'outer: loop {
        for event in main.events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(_), ..
                } => {
                    break 'outer;
                }
                _ => {}
            }
        }

        main.wait_frame();
    }

    main.animate(&mut animation, Animation::FadeDown, 7)?;
    Ok(())
}

enum Animation {
    FadeUp,
    FadeDown,
}

impl MainApp {
    fn animate(
        &mut self,
        texture: &mut Texture,
        animation: Animation,
        steps: usize,
    ) -> Result<(), ApplicationError> {
        // Note that we actually do steps + 1 iteration, as per original behavior
        // Roughly, we do it for half a second for 8 steps. For 60 FPS, which means ~4 frames per step.
        let total_frames = (steps + 1) * 4;

        for idx in 0..=total_frames {
            self.canvas.set_draw_color(Color::RGB(0, 0, 0));
            self.canvas.clear();
            let mut alpha = (255 * idx / total_frames) as u8;
            if let Animation::FadeDown = animation {
                alpha = 255 - alpha;
            }
            texture.set_blend_mode(BlendMode::Blend);
            texture.set_alpha_mod(alpha);
            self.canvas.copy(&texture, None, None).map_err(SdlError)?;

            self.events.pump_events();
            self.canvas.present();
            self.wait_frame();
        }
        Ok(())
    }

    fn wait_frame(&self) {
        // We should wait for the remaining time; for now just do a fixed delay.
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

/// Validate that given path is a valid installation directory.
fn is_valid_installation_directory(s: &std::ffi::OsStr) -> Result<(), std::ffi::OsString> {
    let path = Path::new(s);
    if path.is_dir() && path.join("titlebe.spy").is_file() {
        Ok(())
    } else {
        return Err(format!(
            "'{}' is not a valid game directory (must be a directory with 'titlebe.spy' file).",
            path.display()
        )
        .into());
    }
}
