use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::fonts::Font;
use crate::glyphs::{Glyph, Glyphs};
use crate::options::Options;
use crate::spy::TexturePalette;
use sdl2::keyboard::Scancode;
use sdl2::mixer::Music;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

mod args;
mod context;
mod error;
pub mod fonts;
mod glyphs;
mod options;
pub mod spy;

const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;

//const MAP_WIDTH: usize = 64;
//const MAP_HEIGHT: usize = 45;

pub fn main() -> Result<(), anyhow::Error> {
    let path = args::parse_args();
    let mut ctx = context::ApplicationContext::init(path)?;
    let mut app = Application::init(&ctx)?;
    // To skip menus during development
    if std::env::var("DEV").is_ok() {
        app.options_menu_loop(&mut ctx)?;
    } else {
        app.main_loop(&mut ctx)?;
    }
    Ok(())
}

struct Application {
    title: TexturePalette,
    main_menu: TexturePalette,
    options_menu: TexturePalette,
    info: [TexturePalette; 4],
    codes: TexturePalette,
    glyphs: Glyphs,
    font: Font,
    music1: Music<'static>,
    // Position 465 is position of shop music.
    _music2: Music<'static>,
    options: Options,
}

/// Selected item in the main menu
#[derive(Clone, Copy, PartialEq)]
#[repr(usize)]
enum SelectedMenu {
    NewGame,
    Options,
    Info,
    Quit,
}

impl SelectedMenu {
    /// Get menu item that is next to the current one
    fn next(self) -> SelectedMenu {
        match self {
            SelectedMenu::NewGame => SelectedMenu::Options,
            SelectedMenu::Options => SelectedMenu::Info,
            SelectedMenu::Info => SelectedMenu::Quit,
            SelectedMenu::Quit => SelectedMenu::NewGame,
        }
    }

    /// Get menu item that is previous to the current one
    fn prev(self) -> SelectedMenu {
        self.next().next().next()
    }

    /// Shovel position in the main menu based on the selected item. Should correspond to the main menu texture.
    fn shovel_pos(self) -> (i32, i32) {
        (222, 136 + 48 * self as i32)
    }
}

impl Application {
    fn init(context: &ApplicationContext) -> Result<Self, anyhow::Error> {
        Ok(Self {
            title: context.load_texture("titlebe.spy")?,
            main_menu: context.load_texture("main3.spy")?,
            options_menu: context.load_texture("options5.spy")?,
            glyphs: Glyphs::load(&context)?,
            font: context.load_font("fontti.fon")?,
            info: [
                context.load_texture("info1.spy")?,
                context.load_texture("info3.spy")?,
                context.load_texture("shapet.spy")?,
                context.load_texture("info2.spy")?,
            ],
            codes: context.load_texture("codes.spy")?,
            music1: context.load_music("huippe.s3m")?,
            _music2: context.load_music("oeku.s3m")?,
            options: options::load_options(context.game_dir()),
        })
    }

    fn main_loop(mut self, ctx: &mut ApplicationContext) -> Result<(), anyhow::Error> {
        self.music1.play(-1).map_err(SdlError)?;

        ctx.render_texture(&self.title.texture)?;
        ctx.animate(Animation::FadeUp, 7)?;
        let (scancode, _) = ctx.wait_key_pressed();
        ctx.animate(Animation::FadeDown, 7)?;
        if scancode == Scancode::Escape {
            return Ok(());
        }

        loop {
            if self.main_menu_loop(ctx)? {
                break;
            }

            // Until we can exit
            if true {
                break;
            }
        }
        Ok(())
    }

    /// Returns `true` if exit was selected
    fn main_menu_loop(&mut self, ctx: &mut ApplicationContext) -> Result<bool, anyhow::Error> {
        let mut selected_item = SelectedMenu::NewGame;
        loop {
            self.render_main_menu(ctx, selected_item)?;
            ctx.animate(Animation::FadeUp, 7)?;
            self.main_menu_navigation_loop(ctx, &mut selected_item)?;
            ctx.animate(Animation::FadeDown, 7)?;
            match selected_item {
                SelectedMenu::Quit => return Ok(true),
                SelectedMenu::NewGame => return Ok(false),
                SelectedMenu::Options => self.options_menu_loop(ctx)?,
                SelectedMenu::Info => {
                    self.info_menu(ctx)?;
                }
            }
        }
    }

    /// Runs navigation inside main menu. Return
    fn main_menu_navigation_loop(
        &mut self,
        ctx: &mut ApplicationContext,
        selected: &mut SelectedMenu,
    ) -> Result<(), anyhow::Error> {
        loop {
            let (scancode, _keycode) = ctx.wait_key_pressed();

            match scancode {
                Scancode::Down | Scancode::Kp2 => {
                    let next = selected.next();
                    self.update_shovel(ctx, *selected, next)?;
                    *selected = next;
                }
                Scancode::Up | Scancode::Kp8 => {
                    let prev = selected.prev();
                    self.update_shovel(ctx, *selected, prev)?;
                    *selected = prev;
                }
                Scancode::Escape => {
                    *selected = SelectedMenu::Quit;
                    break;
                }
                Scancode::Kp3 | Scancode::Return | Scancode::Return2 | Scancode::KpEnter => {
                    break;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Display main menu with selected option, plus animation
    fn render_main_menu(
        &mut self,
        ctx: &mut ApplicationContext,
        selected: SelectedMenu,
    ) -> Result<(), anyhow::Error> {
        let texture = &self.main_menu;
        let glyphs = &self.glyphs;
        ctx.with_render_context(|canvas| {
            canvas
                .copy(&texture.texture, None, None)
                .map_err(SdlError)?;
            // Display registered to?
            let (x, y) = selected.shovel_pos();
            glyphs.render(canvas, x, y, Glyph::ShovelPointer)?;
            Ok(())
        })?;
        Ok(())
    }

    fn update_shovel(
        &mut self,
        ctx: &mut ApplicationContext,
        previous: SelectedMenu,
        selected: SelectedMenu,
    ) -> Result<(), anyhow::Error> {
        ctx.with_render_context(|canvas| {
            let (old_x, old_y) = previous.shovel_pos();
            let (w, h) = Glyph::ShovelPointer.dimensions();
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas
                .fill_rect(Rect::new(old_x, old_y, w, h))
                .map_err(SdlError)?;
            let (x, y) = selected.shovel_pos();
            self.glyphs.render(canvas, x, y, Glyph::ShovelPointer)?;
            Ok(())
        })?;
        ctx.present()?;
        Ok(())
    }

    fn info_menu(&mut self, ctx: &mut ApplicationContext) -> Result<(), anyhow::Error> {
        let mut key = Scancode::Escape;
        for info in &self.info {
            ctx.render_texture(&info.texture)?;
            ctx.animate(Animation::FadeUp, 7)?;
            key = ctx.wait_key_pressed().0;
            ctx.animate(Animation::FadeDown, 7)?;
            if key == Scancode::Escape {
                break;
            }
        }
        if key == Scancode::Tab {
            ctx.render_texture(&self.codes.texture)?;
            ctx.animate(Animation::FadeUp, 7)?;
            ctx.wait_key_pressed();
            ctx.animate(Animation::FadeDown, 7)?;
        }
        Ok(())
    }
}
