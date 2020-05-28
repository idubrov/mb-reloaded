use crate::context::ApplicationContext;
use crate::fonts::Font;
use crate::glyphs::Glyphs;
use crate::images::TexturePalette;
use crate::keys::Keys;
use crate::options::Options;
use sdl2::mixer::Music;
use std::path::Path;

mod args;
mod audio;
mod context;
mod error;
pub mod fonts;
mod glyphs;
mod identities;
pub mod images;
mod keys;
mod map;
mod options;
mod players;

mod menu {
  mod keys;
  mod load_levels;
  mod main;
  mod options;
  mod players;
}

const SCREEN_WIDTH: u32 = 640;
const SCREEN_HEIGHT: u32 = 480;

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
      title: ctx.load_spy("TITLEBE.SPY")?,
      main_menu: ctx.load_spy("MAIN3.SPY")?,
      options_menu: ctx.load_spy("OPTIONS5.SPY")?,
      levels_menu: ctx.load_spy("LEVSELEC.SPY")?,
      keys: ctx.load_spy("KEYS.SPY")?,
      glyphs: Glyphs::from_texture(ctx.load_spy("SIKA.SPY")?),
      font: ctx.load_font("FONTTI.FON")?,
      info: [
        ctx.load_spy("INFO1.SPY")?,
        ctx.load_spy("INFO3.SPY")?,
        ctx.load_spy("SHAPET.SPY")?,
        ctx.load_spy("INFO2.SPY")?,
      ],
      codes: ctx.load_spy("CODES.SPY")?,
      players: ctx.load_spy("IDENTIFW.SPY")?,
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
