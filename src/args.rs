use std::path::PathBuf;

pub fn parse_args() -> PathBuf {
    let path = match std::env::args().nth(1) {
        Some(arg) if arg == "--help" => {
            eprintln!("MineBombers 3.11\n");
            eprintln!("USAGE:");
            eprintln!("    mb-reloaded [game-path]");
            std::process::exit(0);
        }
        Some(arg) => PathBuf::from(arg),
        None => match std::env::current_dir() {
            Ok(cur) => cur,
            Err(err) => {
                eprintln!("Cannot detect current directory: {}", err);
                std::process::exit(255);
            }
        },
    };
    if !path.is_dir() || !path.join("TITLEBE.SPY").is_file() {
        eprintln!(
            "'{}' is not a valid game directory (must be a directory with 'TITLEBE.SPY' file).",
            path.display()
        );
        std::process::exit(1);
    }
    path
}
