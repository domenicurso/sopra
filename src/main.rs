mod actions;
mod animation;
mod arguments;
mod cli;
mod config;
mod control;
mod editor;
mod input;
mod palette;
mod prompt;
mod protocol;
mod render;
mod scene;
mod syntax;
mod text;
mod worker;

use std::error::Error;

fn main() {
    if let Err(error) = run() {
        eprintln!("sopra: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    match arguments.next().as_deref().and_then(|value| value.to_str()) {
        Some("--serve") => {
            let socket = arguments.next().ok_or("--serve needs a socket path")?;
            let parent = arguments
                .next()
                .ok_or("--serve needs a parent PID")?
                .to_string_lossy()
                .parse()?;
            if arguments.next().is_some() {
                return Err("unexpected worker arguments".into());
            }
            worker::serve(std::path::Path::new(&socket), parent)
        }
        Some("--buffer" | "--cursor" | "--cwd" | "--secondary" | "--history-file") => {
            worker::once()
        }
        _ => cli::run(std::env::args_os().skip(1).collect()),
    }
}
