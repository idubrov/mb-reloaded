use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::keys::Key;
use crate::menu::preview::generate_preview;
use crate::options::Options;
use crate::world::equipment::Equipment;
use crate::world::map::LevelMap;
use crate::world::player::PlayerComponent;
use crate::Application;
use rand::Rng;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::borrow::Cow;
use std::convert::TryFrom;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShopResult {
  ExitGame,
  Continue,
}

#[derive(Default)]
pub struct Prices {
  prices: [u32; Equipment::TOTAL],
}

struct PlayerState<'a> {
  entity: &'a mut PlayerComponent,
  /// `None` means level exit
  selection: Option<Equipment>,
  ready: bool,
}

struct State<'a> {
  prices: Prices,
  remaining_rounds: u16,
  left: Option<PlayerState<'a>>,
  right: PlayerState<'a>,
}

impl Prices {
  pub fn new(free_market: bool) -> Prices {
    // free market?
    let percentage = if free_market {
      let mut rng = rand::thread_rng();
      130u32 - rng.gen_range(0, 60)
    } else {
      100u32
    };

    let mut prices = Prices::default();
    for equipment in Equipment::all_equipment() {
      prices[equipment] = adjust_price(equipment.base_price(), percentage);
    }
    prices
  }
}

impl std::ops::Index<Equipment> for Prices {
  type Output = u32;

  fn index(&self, index: Equipment) -> &u32 {
    &self.prices[index as usize]
  }
}

impl std::ops::IndexMut<Equipment> for Prices {
  fn index_mut(&mut self, index: Equipment) -> &mut u32 {
    &mut self.prices[index as usize]
  }
}

impl Application<'_> {
  /// Run the shop logic
  pub fn shop(
    &self,
    ctx: &mut ApplicationContext,
    remaining_rounds: u16,
    options: &Options,
    preview_map: Option<&LevelMap>,
    left: Option<&mut PlayerComponent>,
    right: &mut PlayerComponent,
  ) -> Result<ShopResult, anyhow::Error> {
    let mut state = State {
      prices: Prices::new(options.free_market),
      remaining_rounds,
      left: left.map(|entity| PlayerState {
        entity,
        selection: Some(Equipment::SmallBomb),
        ready: false,
      }),
      right: PlayerState {
        entity: right,
        selection: Some(Equipment::SmallBomb),
        ready: false,
      },
    };

    // Render an initial shop screen
    let texture_creator = ctx.texture_creator();
    let palette = &self.shop.palette;
    ctx.with_render_context(|canvas| {
      canvas.copy(&self.shop.texture, None, None).map_err(SdlError)?;
      let remaining = state.remaining_rounds.to_string();
      self.font.render(canvas, 306, 120, palette[1], &remaining)?;

      // Background
      if let Some(left) = &state.left {
        self.render_player_stats(canvas, 0, left)?;
      }
      self.render_player_stats(canvas, 420, &state.right)?;

      // All shop items
      if let Some(left) = &state.left {
        self.render_all_items(canvas, 0, &left, &state.prices)?;
      }
      let right = &state.right;
      self.render_all_items(canvas, 320, &right, &state.prices)?;

      // Preview map
      if let Some(map) = preview_map {
        let tgt = Rect::new(288, 51, 64, 45);
        let preview = generate_preview(map, texture_creator, &self.shop.palette)?;
        canvas.copy(&preview, None, tgt).map_err(SdlError)?;
      }
      Ok(())
    })?;
    ctx.animate(Animation::FadeUp, 7)?;

    let mut result = ShopResult::Continue;
    while state.left.as_ref().map_or(false, |state| !state.ready) || !state.right.ready {
      let scan = ctx.wait_key_pressed().0;
      match scan {
        Scancode::Escape => break,
        Scancode::F10 => {
          result = ShopResult::ExitGame;
          break;
        }
        _ => {}
      }

      if let Some(left) = &mut state.left {
        self.handle_player_keys(ctx, scan, true, options.selling, left, &state.prices)?;
      }
      self.handle_player_keys(ctx, scan, false, options.selling, &mut state.right, &state.prices)?;
    }

    ctx.animate(Animation::FadeDown, 7)?;
    Ok(result)
  }

  fn handle_player_keys(
    &self,
    ctx: &mut ApplicationContext,
    scan: Scancode,
    left: bool,
    selling: bool,
    state: &mut PlayerState,
    prices: &Prices,
  ) -> Result<(), anyhow::Error> {
    let last_selection = state.selection;

    // Left the store already
    if state.ready {
      return Ok(());
    }

    let offset = state.selection.map_or(Equipment::TOTAL as u8, |item| item as u8);
    if Some(scan) == state.entity.keys[Key::Bomb] {
      if let Some(selection) = state.selection {
        if state.entity.cash >= prices[selection] {
          state.entity.cash -= prices[selection];
          state.entity.inventory[selection] += 1;
        }
      } else {
        state.ready = true;
      }
    } else if Some(scan) == state.entity.keys[Key::Choose] {
      if let Some(selection) = state.selection {
        if selling && state.entity.inventory[selection] > 0 {
          // Only return 70% of the cost
          state.entity.cash += (7 * prices[selection] + 5) / 10;
          state.entity.inventory[selection] -= 1;
        }
      }
    } else if Some(scan) == state.entity.keys[Key::Right] {
      state.selection = Equipment::try_from(offset + 1).ok();
    } else if Some(scan) == state.entity.keys[Key::Left] {
      state.selection = Equipment::try_from(offset.max(1) - 1).ok();
    } else if Some(scan) == state.entity.keys[Key::Down] {
      state.selection = Equipment::try_from(offset + 4).ok();
    } else if Some(scan) == state.entity.keys[Key::Up] {
      state.selection = Equipment::try_from(offset.max(4) - 4).ok();
    } else {
      // Nothing to re-render, skip re-rendering
      return Ok(());
    }

    ctx.with_render_context(|canvas| {
      let offsets = if left { (0, 0) } else { (420, 320) };
      self.render_player_stats(canvas, offsets.0, state)?;

      if last_selection != state.selection {
        self.render_shop_slot(canvas, offsets.1, last_selection, state, prices)?;
      }
      self.render_shop_slot(canvas, offsets.1, state.selection, state, prices)?;
      Ok(())
    })?;
    ctx.present()?;
    Ok(())
  }

  fn render_player_stats(
    &self,
    canvas: &mut WindowCanvas,
    offset_x: i32,
    state: &PlayerState,
  ) -> Result<(), anyhow::Error> {
    canvas.set_draw_color(Color::BLACK);

    let palette = &self.shop.palette;
    canvas
      .fill_rect(Rect::new(35 + offset_x, 30, 7 * 8, 8))
      .map_err(SdlError)?;
    canvas
      .fill_rect(Rect::new(35 + offset_x, 44, 7 * 8, 8))
      .map_err(SdlError)?;
    canvas
      .fill_rect(Rect::new(35 + offset_x, 58, 7 * 8, 8))
      .map_err(SdlError)?;

    let power = state.entity.initial_drilling_power();
    self
      .font
      .render(canvas, 35 + offset_x, 16, palette[1], &state.entity.stats.name)?;
    self
      .font
      .render(canvas, 35 + offset_x, 30, palette[3], &power.to_string())?;
    self
      .font
      .render(canvas, 35 + offset_x, 44, palette[5], &state.entity.cash.to_string())?;
    if let Some(item) = state.selection {
      let item_count = state.entity.inventory[item];
      self
        .font
        .render(canvas, 35 + offset_x, 58, palette[1], &item_count.to_string())?;
    }
    Ok(())
  }

  /// `None` for `selected` means that level exit is selected
  fn render_all_items(
    &self,
    canvas: &mut WindowCanvas,
    offset_x: i32,
    state: &PlayerState,
    prices: &Prices,
  ) -> Result<(), anyhow::Error> {
    for slot in Equipment::all_equipment() {
      self.render_shop_slot(canvas, offset_x, Some(slot), state, prices)?;
    }
    self.render_shop_slot(canvas, offset_x, None, state, prices)?;
    Ok(())
  }

  /// `selected` is if slot is currently selected
  fn render_shop_slot(
    &self,
    canvas: &mut WindowCanvas,
    offset_x: i32,
    slot: Option<Equipment>,
    state: &PlayerState,
    prices: &Prices,
  ) -> Result<(), anyhow::Error> {
    let palette = &self.shop.palette;

    let item_index = slot.map(|item| item as usize).unwrap_or(Equipment::TOTAL) as i32;
    let col = item_index % 4;
    let row = item_index / 4;

    let pos_x = col * 64 + 32 + offset_x;
    let pos_y = row * 48 + 96;
    self
      .glyphs
      .render(canvas, pos_x, pos_y, Glyph::ShopSlot(state.selection == slot))?;

    // Render item count
    let item_count = slot.map(|item| state.entity.inventory[item] as i32).unwrap_or(0);
    if item_count != 0 {
      let pos_x = col * 64 + 88 + offset_x;
      let pos_y = row * 48 + 99;
      let delta = 40 - ((item_count * 2).min(40));
      for (idx, color) in [14, 13, 12, 11, 7].iter().copied().enumerate() {
        let idx = idx as i32;
        canvas.set_draw_color(palette[color]);
        canvas
          .draw_line((pos_x + idx, pos_y + delta), (pos_x + idx, pos_y + 41))
          .map_err(SdlError)?;
      }
    }

    // Render item glyph
    let pos_x = col * 64 + 49 + offset_x;
    let pos_y = row * 48 + 99;
    let glyph = slot.map(Glyph::Selection).unwrap_or(Glyph::Ready);
    self.glyphs.render(canvas, pos_x, pos_y, glyph)?;

    // Render item price
    let pos_x = col * 64 + 44 + offset_x;
    let pos_y = row * 48 + 132;

    let text = slot
      .map(|slot| Cow::Owned(format!("{}$", prices[slot])))
      .unwrap_or_else(|| Cow::Borrowed("LEAVE"));
    self.font.render(canvas, pos_x, pos_y, palette[5], &text)?;
    Ok(())
  }
}

fn adjust_price(price: u32, percentage: u32) -> u32 {
  ((price - 1) * percentage + 50) / 100 + 1
}
