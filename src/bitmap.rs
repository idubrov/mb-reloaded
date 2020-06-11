use crate::world::map::MapValue;
use std::convert::{TryFrom, TryInto};

#[macro_export]
macro_rules! bitmap {
  ($bits:expr) => {
    $crate::bitmap::MapValueSet { bits: $bits }
  };
}

/// Bitmap enables indexing
pub struct MapValueSet {
  #[doc(hidden)]
  pub bits: [u8; 32],
}

impl std::fmt::Debug for MapValueSet {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    let mut list: Vec<MapValue> = Vec::new();
    for idx in 0..=255u8 {
      if self[MapValue::try_from(idx).unwrap()] {
        list.push(idx.try_into().unwrap());
      }
    }
    std::fmt::Debug::fmt(&list, f)
  }
}

impl std::ops::Index<MapValue> for MapValueSet {
  type Output = bool;

  fn index(&self, index: MapValue) -> &Self::Output {
    let index = index as usize;
    if (self.bits.as_ref()[index / 8] & (1 << (index & 7))) != 0 {
      &true
    } else {
      &false
    }
  }
}
