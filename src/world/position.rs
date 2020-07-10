use crate::world::map::{MAP_COLS, MAP_ROWS};

/// Facing direction
#[derive(Clone, Copy, PartialEq, Eq)]
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
#[derive(Clone, Copy, PartialEq, Eq)]
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

  pub fn position(self) -> Position {
    Position {
      x: self.col * 10 + 5,
      y: self.row * 10 + 35,
    }
  }
}
