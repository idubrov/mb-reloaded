use crate::context::ApplicationContext;
use crate::fonts::Font;
use crate::glyphs::Glyphs;
use crate::keys::Keys;
use crate::options::Options;
use crate::spy::TexturePalette;
use sdl2::mixer::Music;
use std::path::Path;

mod args;
mod context;
mod error;
pub mod fonts;
mod glyphs;
mod keys;
mod map;
mod options;
pub mod spy;

mod menu {
    mod keys;
    mod load_levels;
    mod main;
    mod options;
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
            app.load_levels(&mut ctx)?;
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
            title: ctx.load_texture("titlebe.spy")?,
            main_menu: ctx.load_texture("main3.spy")?,
            options_menu: ctx.load_texture("options5.spy")?,
            levels_menu: ctx.load_texture("levselec.spy")?,
            keys: ctx.load_texture("keys.spy")?,
            glyphs: Glyphs::from_texture(ctx.load_texture("sika.spy")?),
            font: ctx.load_font("fontti.fon")?,
            info: [
                ctx.load_texture("info1.spy")?,
                ctx.load_texture("info3.spy")?,
                ctx.load_texture("shapet.spy")?,
                ctx.load_texture("info2.spy")?,
            ],
            codes: ctx.load_texture("codes.spy")?,
            music1: ctx.load_music("huippe.s3m")?,
            _music2: ctx.load_music("oeku.s3m")?,
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
