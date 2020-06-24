use crate::bitmap;
use crate::bitmap::MapValueSet;

/// Bitmap of which map values are exposing border of surrounding dirt and stones
pub const DIRT_BORDER_BITMAP: MapValueSet = bitmap!([
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0001,
  0b0000_0000,
  0b0000_0100,
  0b0000_0000,
  0b1000_0000,
  0b1000_0011,
  0b1111_1000,
  0b0011_1111,
  0b1000_1000,
  0b1111_0011,
  0b0000_1111,
  0b1111_1100,
  0b1111_1111,
  0b1111_0111,
  0b1111_1111,
  0b1000_1111,
  0b0011_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
]);

pub const PUSHABLE_BITMAP: MapValueSet = bitmap!([
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0100,
  0b0000_0000,
  0b1000_0000,
  0b0000_0011,
  0b1001_1000,
  0b0000_0111,
  0b1000_0000,
  0b1111_0001,
  0b0000_1111,
  0b0111_1100,
  0b0000_0000,
  0b1110_0000,
  0b0001_1110,
  0b0000_1100,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
]);

pub const CANNOT_PLACE_BOMB: MapValueSet = bitmap!([
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0010,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b1000_0000,
  0b0000_0011,
  0b1011_1000,
  0b0001_1111,
  0b1000_0000,
  0b1111_0001,
  0b0000_1111,
  0b0111_1100,
  0b0000_0000,
  0b1111_0000,
  0b1111_1110,
  0b0000_1111,
  0b0011_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
  0b0000_0000,
]);
