//! Player statistics
use byteorder::{LittleEndian, ReadBytesExt};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Failed to load high scores from '{path}'")]
pub struct ScoresLoadError {
  #[source]
  source: std::io::Error,
  path: PathBuf,
}

#[derive(Debug, Error)]
#[error("Failed to save high scores to '{path}'")]
pub struct ScoresSaveError {
  #[source]
  source: std::io::Error,
  path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Score {
  pub name: String,
  pub level: u8,
  pub cash: u32,
}

#[derive(Default, Debug)]
pub struct Highscores {
  pub scores: Box<[Option<Score>; 10]>,
}

impl Highscores {
  /// Load high scoress from `HIGHSCOR.DAT`.
  pub fn load(game_dir: &Path) -> Result<Highscores, ScoresLoadError> {
    let path = game_dir.join("HIGHSCOR.DAT");
    if path.is_file() {
      Highscores::load_scores_internal(&path).map_err(|source| ScoresLoadError { path, source })
    } else {
      Ok(Highscores::default())
    }
  }

  fn load_scores_internal(path: &Path) -> Result<Highscores, std::io::Error> {
    let data = std::fs::read(path)?;
    let mut players = Highscores::default();
    // Invalid format, just ignore
    if data.len() != 260 {
      return Ok(players);
    }

    for player in 0..10 {
      // Each record is 26 byte long
      let data = &data[player * 26..][..26];
      let len = usize::from(data[0].min(20));
      if len > 2 {
        // Strip first two characters, original game always has "1 " in them (they seem to be adding
        // player index to the player name, and in a single player game, player index is always 1).
        let name = String::from_utf8_lossy(&data[3..3 + len - 2]).into_owned();
        let mut it = &data[21..26];
        let level = it.read_u8().unwrap();
        let cash = it.read_u32::<LittleEndian>().unwrap();
        players.scores[player] = Some(Score { name, level, cash });
      }
    }

    Ok(players)
  }

  pub fn save(&self, game_dir: &Path) -> Result<(), ScoresSaveError> {
    let mut out: Vec<u8> = Vec::with_capacity(32 * 101);
    for score in self.scores.iter() {
      if let Some(score) = score {
        let name_len = score.name.len().min(18);
        // See `load`, we always add "1 " in the front.
        out.push((name_len + 2) as u8);
        out.push(b'1');
        out.push(b' ');
        out.extend_from_slice(&score.name.as_bytes()[..name_len]);
        out.resize(out.len() + (18 - name_len), 0);
        out.push(score.level);
        out.extend_from_slice(&score.cash.to_le_bytes());
      } else {
        // Just pad zeroes
        out.resize(out.len() + 26, 0);
      }
    }
    assert_eq!(26 * 10, out.len());

    let path = game_dir.join("HIGHSCOR.DAT");
    std::fs::write(&path, &out).map_err(|source| ScoresSaveError { path, source })?;
    Ok(())
  }
}

#[test]
fn test() {
  let scores = Highscores::load(Path::new("/Users/idubrov/DOS Games/Mb 311.boxer/C.harddisk/mb311")).unwrap();
  scores.save(Path::new("/tmp/")).unwrap();
  eprintln!("{:#?}", scores);
}
