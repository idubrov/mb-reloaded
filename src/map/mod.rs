mod fog;
mod hits;
mod level;
mod timer;

pub const MAP_ROWS: usize = 45;
pub const MAP_COLS: usize = 64;

pub use fog::FogMap;
pub use hits::HitsMap;
pub use level::{Cursor, InvalidMap, LevelInfo, LevelMap, MapValue};
pub use timer::TimerMap;
