use crate::context::{Animation, ApplicationContext};
use crate::error::ApplicationError::SdlError;
use crate::glyphs::Glyph;
use crate::map::LevelMap;
use crate::menu::preview::generate_preview;
use crate::player::{ActivePlayer, Equipment, Inventory};
use crate::Application;
use rand::Rng;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::borrow::Cow;

#[derive(Default)]
pub struct Prices {
  prices: [u32; Equipment::TOTAL],
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
  pub fn shop(
    &self,
    ctx: &mut ApplicationContext,
    remaining_rounds: u16,
    free_market: bool,
    preview_map: Option<&LevelMap>,
    left: Option<&mut ActivePlayer>,
    right: &mut ActivePlayer,
  ) -> Result<(), anyhow::Error> {
    let prices = Prices::new(free_market);

    let preview_texture = preview_map
      .map(|map| generate_preview(map, ctx.texture_creator(), &self.shop.palette))
      .transpose()?;

    let palette = &self.shop.palette;
    ctx.with_render_context(|canvas| {
      canvas.copy(&self.shop.texture, None, None).map_err(SdlError)?;
      self
        .font
        .render(canvas, 306, 120, palette[1], &remaining_rounds.to_string())?;

      if let Some(ref left) = left {
        let power = left.drilling_power + left.base_drillingpower;
        self.font.render(canvas, 35, 30, palette[3], &power.to_string())?;
        self.font.render(canvas, 35, 16, palette[1], &left.player.name)?;
        self.font.render(canvas, 35, 44, palette[5], &left.cash.to_string())?;
        let item_count = right.inventory[Equipment::SmallBomb];
        self.font.render(canvas, 35, 58, palette[1], &item_count.to_string())?;
        self.render_items(canvas, 0, &left.inventory, &prices, Some(Equipment::SmallBomb))?;
      }
      let power = right.drilling_power + right.base_drillingpower;
      self.font.render(canvas, 455, 30, palette[3], &power.to_string())?;
      self.font.render(canvas, 455, 16, palette[1], &right.player.name)?;
      self.font.render(canvas, 455, 44, palette[5], &right.cash.to_string())?;
      let item_count = right.inventory[Equipment::SmallBomb];
      self.font.render(canvas, 455, 58, palette[1], &item_count.to_string())?;
      self.render_items(canvas, 320, &right.inventory, &prices, Some(Equipment::SmallBomb))?;

      if let Some(preview) = preview_texture {
        let tgt = Rect::new(288, 51, 64, 45);
        canvas.copy(&preview, None, tgt).map_err(SdlError)?;
      }
      Ok(())
    })?;
    ctx.animate(Animation::FadeUp, 7)?;

    Ok(())
  }

  /// `None` for `selected` means that level exit is selected
  fn render_items(
    &self,
    canvas: &mut WindowCanvas,
    offset_x: i32,
    inventory: &Inventory,
    prices: &Prices,
    selected: Option<Equipment>,
  ) -> Result<(), anyhow::Error> {
    for slot in Equipment::all_equipment() {
      self.render_shop_slot(canvas, offset_x, Some(slot), inventory, prices, selected == Some(slot))?;
    }
    self.render_shop_slot(canvas, offset_x, None, inventory, prices, selected.is_none())?;
    Ok(())
  }

  /// `selected` is if slot is currently selected
  fn render_shop_slot(
    &self,
    canvas: &mut WindowCanvas,
    offset_x: i32,
    slot: Option<Equipment>,
    inventory: &Inventory,
    prices: &Prices,
    selected: bool,
  ) -> Result<(), anyhow::Error> {
    let palette = &self.shop.palette;

    let item_index = slot.map(|item| item as usize).unwrap_or(Equipment::TOTAL) as i32;
    let col = item_index % 4;
    let row = item_index / 4;

    let pos_x = col * 64 + 32 + offset_x;
    let pos_y = row * 48 + 96;
    self.glyphs.render(canvas, pos_x, pos_y, Glyph::ShopSlot(selected))?;

    // Render item count
    let item_count = slot.map(|item| inventory[item] as i32).unwrap_or(0);
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
