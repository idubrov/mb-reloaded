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
