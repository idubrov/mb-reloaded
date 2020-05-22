use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::{Glyph, Glyphs};
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Texture;

mod args;
mod context;
mod error;
mod glyphs;
pub mod spy;

const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;

//const MAP_WIDTH: usize = 64;
//const MAP_HEIGHT: usize = 45;

pub fn main() -> Result<(), anyhow::Error> {
    let path = args::parse_args();
    let context = context::ApplicationContext::init(path)?;
    let app = Application::init(context)?;
    app.main_loop()?;
    Ok(())
}

struct Application {
    title: Texture,
    menu: Texture,
    info: [Texture; 4],
    codes: Texture,
    glyphs: Glyphs,
    context: ApplicationContext,
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
    fn init(context: ApplicationContext) -> Result<Self, anyhow::Error> {
        Ok(Self {
            title: context.load_texture("titlebe.spy")?,
            menu: context.load_texture("main3.spy")?,
            glyphs: Glyphs::load(&context)?,
            info: [
                context.load_texture("info1.spy")?,
                context.load_texture("info3.spy")?,
                context.load_texture("shapet.spy")?,
                context.load_texture("info2.spy")?,
            ],
            codes: context.load_texture("codes.spy")?,
            context,
        })
    }

    fn main_loop(mut self) -> Result<(), anyhow::Error> {
        self.context.render_texture(&self.title)?;
        self.context.animate(Animation::FadeUp, 7)?;
        let key = self.context.wait_key_pressed();
        self.context.animate(Animation::FadeDown, 7)?;
        if key == Scancode::Escape {
            return Ok(());
        }

        loop {
            if self.main_menu_loop()? {
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
    fn main_menu_loop(&mut self) -> Result<bool, anyhow::Error> {
        let mut selected_item = SelectedMenu::NewGame;
        loop {
            self.enter_main_menu(selected_item)?;
            self.main_menu_navigation_loop(&mut selected_item)?;
            self.context.animate(Animation::FadeDown, 7)?;
            match selected_item {
                SelectedMenu::Quit => return Ok(true),
                SelectedMenu::NewGame => return Ok(false),
                SelectedMenu::Options => {}
                SelectedMenu::Info => {
                    self.info_menu()?;
                }
            }
        }
    }

    /// Runs navigation inside main menu. Return
    fn main_menu_navigation_loop(
        &mut self,
        selected: &mut SelectedMenu,
    ) -> Result<(), anyhow::Error> {
        loop {
            let key = self.context.wait_key_pressed();

            match key {
                Scancode::Down | Scancode::Kp2 => {
                    let next = selected.next();
                    self.update_shovel(*selected, next)?;
                    *selected = next;
                }
                Scancode::Up | Scancode::Kp8 => {
                    let prev = selected.prev();
                    self.update_shovel(*selected, prev)?;
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
    fn enter_main_menu(&mut self, selected: SelectedMenu) -> Result<(), anyhow::Error> {
        let menu = &self.menu;
        let glyphs = &self.glyphs;
        self.context.with_render_context(|canvas| {
            canvas.copy(menu, None, None).map_err(SdlError)?;
            // Display registered to?
            let (x, y) = selected.shovel_pos();
            glyphs.render(canvas, Glyph::Shovel, x, y)?;
            Ok(())
        })?;
        self.context.animate(Animation::FadeUp, 7)?;
        Ok(())
    }

    fn update_shovel(
        &mut self,
        previous: SelectedMenu,
        selected: SelectedMenu,
    ) -> Result<(), anyhow::Error> {
        let glyphs = &self.glyphs;
        self.context.with_render_context(|canvas| {
            let (old_x, old_y) = previous.shovel_pos();
            let (w, h) = Glyph::Shovel.dimensions();
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas
                .fill_rect(Rect::new(old_x, old_y, w, h))
                .map_err(SdlError)?;
            let (x, y) = selected.shovel_pos();
            glyphs.render(canvas, Glyph::Shovel, x, y)?;
            Ok(())
        })?;
        self.context.present()?;
        Ok(())
    }

    fn info_menu(&mut self) -> Result<(), anyhow::Error> {
        let mut key = Scancode::Escape;
        for info in &self.info {
            self.context.render_texture(info)?;
            self.context.animate(Animation::FadeUp, 7)?;
            key = self.context.wait_key_pressed();
            self.context.animate(Animation::FadeDown, 7)?;
            if key == Scancode::Escape {
                break;
            }
        }
        if key == Scancode::Tab {
            self.context.render_texture(&self.codes)?;
            self.context.animate(Animation::FadeUp, 7)?;
            self.context.wait_key_pressed();
            self.context.animate(Animation::FadeDown, 7)?;
        }
        Ok(())
    }
}
