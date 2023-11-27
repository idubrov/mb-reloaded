use std::path::PathBuf;

pub struct Args {
  pub path: PathBuf,
  pub campaign_mode: bool,
}

pub fn parse_args() -> Args {
  let mut args = Args {
    path: Default::default(),
    campaign_mode: false,
  };
  for arg in std::env::args().skip(1) {
    match arg.as_str() {
      "--campaign" => {
        args.campaign_mode = true;
      }
      "--help" => {
        eprintln!("MineBombers 3.11\n");
        eprintln!("USAGE:");
        eprintln!("    mb-reloaded [--campaign] [game-path]");
        std::process::exit(0);
      }
      arg => {
        args.path = PathBuf::from(arg);
      }
    }
  }
  if args.path.as_os_str().len() == 0 {
    args.path = match std::env::current_dir() {
      Ok(cur) => cur,
      Err(err) => {
        eprintln!("Cannot detect current directory: {}", err);
        std::process::exit(255);
      }
    }
  }

  if !args.path.is_dir() || !args.path.join("TITLEBE.SPY").is_file() {
    eprintln!(
      "'{}' is not a valid game directory (must be a directory with 'TITLEBE.SPY' file).",
      args.path.display()
    );
    std::process::exit(1);
  }
  args
}
