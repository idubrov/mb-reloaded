use crate::context::ApplicationContext;
use crate::entity::{PlayerEntity, PlayerInfo};
use crate::fonts::Font;
use crate::glyphs::Glyphs;
use crate::images::TexturePalette;
use crate::map::LevelInfo;
use crate::settings::GameSettings;
use sdl2::mixer::Music;
use std::path::Path;

mod args;
mod audio;
mod context;
mod entity;
mod error;
pub mod fonts;
mod glyphs;
mod identities;
pub mod images;
mod keys;
pub mod map;
mod menu;
mod options;
mod roster;
mod settings;

const SCREEN_WIDTH: u32 = 640;
const SCREEN_HEIGHT: u32 = 480;

//const MAP_WIDTH: usize = 64;
//const MAP_HEIGHT: usize = 45;

pub fn main() -> Result<(), anyhow::Error> {
  let path = args::parse_args();
  context::ApplicationContext::with_context(path, |mut ctx| {
    let app = Application::init(&ctx)?;
    // To skip menus during development
    if std::env::var("DEV").is_ok() {
      let settings = GameSettings::load(ctx.game_dir());
      let player1 = PlayerInfo {
        roster_index: 0,
        name: "First".to_string(),
      };
      let player2 = PlayerInfo {
        roster_index: 0,
        name: "Second".to_string(),
      };
      let mut players = vec![
        PlayerEntity::new(player1, settings.keys.keys[0], u32::from(settings.options.cash)),
        PlayerEntity::new(player2, settings.keys.keys[1], u32::from(settings.options.cash)),
      ];
      app.play_round(&mut ctx, &mut players, 0, &LevelInfo::Random, &settings)?;
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
  select_players: TexturePalette<'t>,
  shop: TexturePalette<'t>,
  players: TexturePalette<'t>,
  glyphs: Glyphs<'t>,
  font: Font<'t>,
  music1: Music<'static>,
  // Position 465 is position of shop music.
  music2: Music<'static>,
  registered: String,
}

impl<'textures> Application<'textures> {
  fn init(ctx: &ApplicationContext<'_, 'textures>) -> Result<Self, anyhow::Error> {
    Ok(Self {
      title: ctx.load_spy("TITLEBE.SPY")?,
      main_menu: ctx.load_spy("MAIN3.SPY")?,
      options_menu: ctx.load_spy("OPTIONS5.SPY")?,
      levels_menu: ctx.load_spy("LEVSELEC.SPY")?,
      keys: ctx.load_spy("KEYS.SPY")?,
      shop: ctx.load_spy("SHOPPIC.SPY")?,
      glyphs: Glyphs::from_texture(ctx.load_spy("SIKA.SPY")?),
      font: ctx.load_font("FONTTI.FON")?,
      info: [
        ctx.load_spy("INFO1.SPY")?,
        ctx.load_spy("INFO3.SPY")?,
        ctx.load_spy("SHAPET.SPY")?,
        ctx.load_spy("INFO2.SPY")?,
      ],
      codes: ctx.load_spy("CODES.SPY")?,
      select_players: ctx.load_spy("IDENTIFW.SPY")?,
      players: ctx.load_spy("PLAYERS.SPY")?,
      music1: ctx.load_music("HUIPPE.S3M")?,
      music2: ctx.load_music("OEKU.S3M")?,
      registered: load_registered(ctx.game_dir()).unwrap_or_else(String::new),
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
