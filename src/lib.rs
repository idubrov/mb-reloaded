use crate::context::ApplicationContext;
use crate::effects::SoundEffects;
use crate::fonts::Font;
use crate::glyphs::Glyphs;
use crate::images::TexturePalette;
use crate::roster::RosterInfo;
use crate::settings::GameSettings;
use crate::world::equipment::Equipment;
use crate::world::map::{LevelInfo, LevelMap};
use crate::world::player::PlayerComponent;
use sdl2::mixer::Music;
use std::path::Path;

mod args;
pub mod bitmap;
mod context;
pub mod effects;
mod error;
pub mod fonts;
mod glyphs;
mod identities;
pub mod images;
mod keys;
mod menu;
mod options;
mod roster;
mod settings;
pub mod world;

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
      // let data = std::fs::read("../minebomb/KARJAISU.VOC")?;
      // let audio = crate::audio::from_voc_bytes(data, Some(10300))?;
      sdl2::mixer::allocate_channels(16);
      // sdl2::mixer::Channel(0).play(&audio, 10);

      let data = std::fs::read("../minebomb/AA1.MNE")?;
      let map = LevelMap::from_file_map(data)?;
      let level = LevelInfo::File {
        name: "CARAMBA".into(),
        map,
      };
      let settings = GameSettings::load(ctx.game_dir());
      let player1 = RosterInfo {
        name: "First".to_string(),
        ..Default::default()
      };
      let player2 = RosterInfo {
        name: "Second".to_string(),
        ..Default::default()
      };
      let mut players = vec![
        PlayerComponent::new(player1, settings.keys.keys[0], &settings.options),
        PlayerComponent::new(player2, settings.keys.keys[1], &settings.options),
        // PlayerEntity::new(player3, settings.keys.keys[1], u32::from(settings.options.cash)),
        // PlayerEntity::new(player4, settings.keys.keys[1], u32::from(settings.options.cash)),
      ];
      // for item in Equipment::all_equipment() {
      //   if item != Equipment::Armor {
      //     players[0].inventory[item] = 50;
      //   } else {
      //     players[0].inventory[item] = 1;
      //   }
      // }
      players[0].inventory[Equipment::Clone] = 100;
      players[0].inventory[Equipment::Flamethrower] = 100;
      //players[1].inventory[Equipment::SmallBomb] = 100;
      //players[0].inventory[Equipment::Napalm] = 100;
      //players[0].inventory[Equipment::AtomicBomb] = 100;
      //players[0].inventory[Equipment::Armor] = 100;
      //players[0].inventory[Equipment::LargePickaxe] = 0;
      players[0].inventory[Equipment::Drill] = 100;
      players[1].inventory[Equipment::Drill] = 100;
      app.play_round(&mut ctx, &mut players, 0, &level, &settings)?;
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
  effects: SoundEffects,
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
      effects: SoundEffects::new(ctx.game_dir())?,
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
