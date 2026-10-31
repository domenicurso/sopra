mod actions;
mod animation;
mod arguments;
mod config;
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    match arguments.next().as_deref().and_then(|value| value.to_str()) {
        Some("--serve") => {
            let socket = arguments.next().ok_or("--serve needs a socket path")?;
            let parent = arguments
                .next()
                .ok_or("--serve needs a parent PID")?
                .to_string_lossy()
                .parse()?;
            worker::serve(std::path::Path::new(&socket), parent)
        }
        _ => worker::once(),
    }
}
