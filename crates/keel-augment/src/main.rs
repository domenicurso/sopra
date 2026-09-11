use std::{env, process::ExitCode};

use keel_augment::{parse_one_shot_args, render_snapshot, run_server};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    if matches!(args.next().as_deref(), Some("--server")) {
        return match run_server() {
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("keel-augment: {error}");
                ExitCode::from(1)
            }
        };
    }

    let args = env::args().skip(1);
    match parse_one_shot_args(args) {
        Ok(snapshot) => {
            print!("{}", render_snapshot(snapshot));
            ExitCode::SUCCESS
        }
        Err(message) if message == "help" => {
            println!(
                "usage: keel-augment --buffer TEXT --cursor N --width N [--rows N] [--keymap NAME] [--status N]"
            );
            println!("       keel-augment --server");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("keel-augment: {message}");
            ExitCode::from(2)
        }
    }
}
