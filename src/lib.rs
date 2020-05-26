use crate::context::ApplicationContext;
use crate::fonts::Font;
use crate::glyphs::Glyphs;
use crate::keys::Keys;
use crate::options::Options;
use crate::spy::TexturePalette;
use sdl2::mixer::Music;
use std::path::Path;

mod args;
mod audio;
mod context;
mod error;
pub mod fonts;
mod glyphs;
mod identities;
mod keys;
mod map;
mod options;
mod players;
pub mod spy;

mod menu {
  mod keys;
  mod load_levels;
  mod main;
  mod options;
  mod players;
}

const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;

//const MAP_WIDTH: usize = 64;
//const MAP_HEIGHT: usize = 45;

pub fn main() -> Result<(), anyhow::Error> {
  let path = args::parse_args();
  context::ApplicationContext::with_context(path, |mut ctx| {
    let mut app = Application::init(&ctx)?;
    // To skip menus during development
    if std::env::var("DEV").is_ok() {
      app.players_select_menu(&mut ctx)?;
    } else {
      app.main_menu(&mut ctx)?;
    }
    Ok(())
  })?;
  Ok(())
}

struct Application<'t> {
  title: TexturePalette<'t>,
  main_menu: TexturePalette<'t>,
  options_menu: TexturePalette<'t>,
  levels_menu: TexturePalette<'t>,
  info: [TexturePalette<'t>; 4],
  keys: TexturePalette<'t>,
  codes: TexturePalette<'t>,
  players: TexturePalette<'t>,
  glyphs: Glyphs<'t>,
  font: Font<'t>,
  music1: Music<'static>,
  // Position 465 is position of shop music.
  _music2: Music<'static>,
  options: Options,
  registered: String,
  player_keys: [Keys; 4],
}

impl<'textures> Application<'textures> {
  fn init(ctx: &ApplicationContext<'_, 'textures>) -> Result<Self, anyhow::Error> {
    let player_keys = keys::load_keys(ctx.game_dir());
    Ok(Self {
      title: ctx.load_texture("TITLEBE.SPY")?,
      main_menu: ctx.load_texture("MAIN3.SPY")?,
      options_menu: ctx.load_texture("OPTIONS5.SPY")?,
      levels_menu: ctx.load_texture("LEVSELEC.SPY")?,
      keys: ctx.load_texture("KEYS.SPY")?,
      glyphs: Glyphs::from_texture(ctx.load_texture("SIKA.SPY")?),
      font: ctx.load_font("FONTTI.FON")?,
      info: [
        ctx.load_texture("INFO1.SPY")?,
        ctx.load_texture("INFO3.SPY")?,
        ctx.load_texture("SHAPET.SPY")?,
        ctx.load_texture("INFO2.SPY")?,
      ],
      codes: ctx.load_texture("CODES.SPY")?,
      players: ctx.load_texture("IDENTIFW.SPY")?,
      music1: ctx.load_music("HUIPPE.S3M")?,
      _music2: ctx.load_music("OEKU.S3M")?,
      options: options::load_options(ctx.game_dir()),
      registered: load_registered(ctx.game_dir()).unwrap_or_else(String::new),
      player_keys,
    })
  }
}

fn load_registered(path: &Path) -> Option<String> {
  let register = std::fs::read(path.join("register.dat")).ok()?;
  if register.is_empty() {
    return None;
  }
  let len = register[0] as usize;
  if len < 26 && len < register.len() + 1 {
    Some(String::from_utf8_lossy(&register[1..1 + len]).into_owned())
  } else {
    None
  }
}
