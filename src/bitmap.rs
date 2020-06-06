use crate::map::MapValue;
use std::convert::TryInto;

#[macro_export]
macro_rules! bitmap {
  ($bits:expr) => {
    Bitmap { bits: $bits }
  };
}

/// Bitmap enables indexing
pub struct Bitmap<A: AsRef<[u8]>> {
  #[doc(hidden)]
  pub bits: A,
}

impl std::fmt::Debug for Bitmap<[u8; 32]> {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    let mut list: Vec<MapValue> = Vec::new();
    for idx in 0..=255u8 {
      if self[usize::from(idx)] {
        list.push(idx.try_into().unwrap());
      }
    }
    std::fmt::Debug::fmt(&list, f)
  }
}

impl<A: AsRef<[u8]>> std::ops::Index<usize> for Bitmap<A> {
  type Output = bool;

  fn index(&self, index: usize) -> &Self::Output {
    if (self.bits.as_ref()[index / 8] & (1 << (index & 7))) != 0 {
      &true
    } else {
      &false
    }
  }
}
