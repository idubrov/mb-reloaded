use crate::world::map::{MAP_COLS, MAP_ROWS};

/// Facing direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
  Right,
  Left,
  Up,
  Down,
}

impl Direction {
  /// Iterate through all the directions
  pub fn all() -> impl Iterator<Item = Direction> {
    [Direction::Right, Direction::Left, Direction::Up, Direction::Down]
      .iter()
      .copied()
  }

  /// Reverse the direction
  pub fn reverse(self) -> Self {
    match self {
      Direction::Left => Direction::Right,
      Direction::Right => Direction::Left,
      Direction::Up => Direction::Down,
      Direction::Down => Direction::Up,
    }
  }

  /// Ortho direction (turn 90 degrees to the right)
  pub fn ortho(self) -> Self {
    match self {
      Direction::Left => Direction::Up,
      Direction::Up => Direction::Right,
      Direction::Right => Direction::Down,
      Direction::Down => Direction::Left,
    }
  }
}

/// Position on the map. Center of the row 0, column 0 is (x = 5; y = 35)
#[derive(Clone, Copy)]
pub struct Position {
  pub x: u16,
  pub y: u16,
}

impl Position {
  pub fn new(x: u16, y: u16) -> Self {
    Self { x, y }
  }

  /// Adjust coordinate to step in a given direction
  pub fn step(&mut self, dir: Direction) {
    match dir {
      Direction::Left => self.x -= 1,
      Direction::Right => self.x += 1,
      Direction::Up => self.y -= 1,
      Direction::Down => self.y += 1,
    }
  }

  /// Center the coordinate orthogonal to the moving direction
  pub fn center_orthogonal(&mut self, dir: Direction) {
    match dir {
      Direction::Left | Direction::Right => {
        self.y = (self.y / 10) * 10 + 5;
      }
      Direction::Up | Direction::Down => {
        self.x = (self.x / 10) * 10 + 5;
      }
    }
  }

  /// Convert position on the map to map cell coordinate
  pub fn cursor(self) -> Cursor {
    let row = ((self.y - 30) / 10) as usize;
    let col = (self.x / 10) as usize;
    Cursor::new(row as u16, col as u16)
  }
}

impl From<Cursor> for Position {
  fn from(cursor: Cursor) -> Self {
    cursor.position()
  }
}

impl From<Position> for Cursor {
  fn from(pos: Position) -> Self {
    pos.cursor()
  }
}

/// Map cell coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
  pub row: u16,
  pub col: u16,
}

impl Cursor {
  pub fn new(row: u16, col: u16) -> Cursor {
    Cursor { row, col }
  }

  /// Return a new cursor that is offset from the current one by one step in a given direction.
  /// If cannot go further in that direction, return the same value.
  pub fn to(self, dir: Direction) -> Cursor {
    let (row, col) = match dir {
      // Check boundaries
      Direction::Left if self.col == 0 => (self.row, self.col),
      Direction::Right if self.col == MAP_COLS - 1 => (self.row, self.col),
      Direction::Up if self.row == 0 => (self.row, self.col),
      Direction::Down if self.row == MAP_ROWS - 1 => (self.row, self.col),

      Direction::Left => (self.row, self.col - 1),
      Direction::Right => (self.row, self.col + 1),
      Direction::Up => (self.row - 1, self.col),
      Direction::Down => (self.row + 1, self.col),
    };
    Cursor { row, col }
  }

  /// Offset given cursor by given delta; returns `None` if hits border of the map or outside of the map.
  pub fn offset(self, delta_row: i16, delta_col: i16) -> Option<Cursor> {
    let row = (self.row as i16) + delta_row;
    let col = (self.col as i16) + delta_col;
    if row > 0 && row < (MAP_ROWS - 1) as i16 && col > 0 && col < (MAP_COLS - 1) as i16 {
      Some(Cursor::new(row as u16, col as u16))
    } else {
      None
    }
  }

  /// Offset given cursor by given delta; clamps the values to the area inside the map.
  pub fn offset_clamp(self, delta_row: i16, delta_col: i16) -> Cursor {
    let row = ((self.row as i16) + delta_row).max(0).min((MAP_ROWS - 1) as i16);
    let col = ((self.col as i16) + delta_col).max(0).min((MAP_COLS - 1) as i16);
    Cursor::new(row as u16, col as u16)
  }

  /// Find absolute distance in both directions to a given target
  pub fn distance(self, other: Cursor) -> (u16, u16) {
    let delta_col = if self.col > other.col {
      self.col - other.col
    } else {
      other.col - self.col
    };
    let delta_row = if self.row > other.row {
      self.row - other.row
    } else {
      other.row - self.row
    };
    (delta_row, delta_col)
  }

  /// Iterate through all map cells (including the border ones)
  pub fn all() -> impl Iterator<Item = Cursor> {
    (0..MAP_ROWS)
      .flat_map(|row| (0..MAP_COLS).map(move |col| (row, col)))
      .map(|(row, col)| Cursor::new(row, col))
  }

  /// Iterate through all map cells (excluding the border ones)
  pub fn all_without_borders() -> impl Iterator<Item = Cursor> {
    (1..MAP_ROWS - 1)
      .flat_map(|row| (1..MAP_COLS - 1).map(move |col| (row, col)))
      .map(|(row, col)| Cursor::new(row, col))
  }

  /// Check if this cursor is pointing at a border cell
  pub fn is_on_border(self) -> bool {
    self.row == 0 || self.row == (MAP_ROWS - 1) || self.col == 0 || self.col == (MAP_COLS - 1)
  }

  pub fn position(self) -> Position {
    Position {
      x: self.col * 10 + 5,
      y: self.row * 10 + 35,
    }
  }
}
