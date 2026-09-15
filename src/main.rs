mod editor;
mod input;
mod render;
mod scene;

use std::{
    env,
    error::Error,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use editor::{EditorConfig, EditorState, ExitReason, RunResult};
use input::{CursorPosition, Terminal};

#[derive(Debug)]
struct Args {
    buffer: String,
    cursor: usize,
    prompt: String,
    result_prefix: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut buffer = String::new();
        let mut cursor = 0;
        let mut prompt = "keel-demo ❯ ".to_string();
        let mut result_prefix = None;
        let mut arguments = env::args_os().skip(1);

        while let Some(argument) = arguments.next() {
            match argument.to_string_lossy().as_ref() {
                "--buffer" => buffer = next_value(&mut arguments, "--buffer")?,
                "--cursor" => {
                    cursor = next_value(&mut arguments, "--cursor")?
                        .parse()
                        .map_err(|_| "--cursor must be a character index".to_string())?;
                }
                "--prompt" => prompt = next_value(&mut arguments, "--prompt")?,
                "--result-prefix" => {
                    result_prefix = Some(PathBuf::from(next_value(
                        &mut arguments,
                        "--result-prefix",
                    )?));
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}")),
            }
        }

        Ok(Self {
            buffer,
            cursor,
            prompt,
            result_prefix,
        })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse().map_err(|message| format!("keel-demo: {message}"))?;
    let mut terminal = Terminal::open()?;
    let size = terminal.size()?;
    // ZLE has already positioned the terminal on the active line. The renderer saves that
    // position and treats it as row zero, so the prototype does not need terminal cursor queries.
    let anchor = CursorPosition { row: 0, column: 0 };
    let mut editor = EditorState::new(EditorConfig {
        buffer: args.buffer,
        cursor_chars: args.cursor,
        prompt: args.prompt,
        anchor,
        size,
    });
    let result = editor.run(&mut terminal);

    if result.is_err() {
        let _ = terminal.write_all(b"\x1b[0m\x1b[?25h");
        let _ = terminal.flush();
    }
    terminal.restore()?;

    let result = result?;
    if let Some(prefix) = args.result_prefix {
        write_result(&prefix, &result)?;
    } else {
        match result.reason {
            ExitReason::Accepted => println!("{}", result.buffer),
            ExitReason::Cancelled => println!("cancelled"),
            ExitReason::Interrupted => println!("interrupted"),
        }
    }
    Ok(())
}

fn next_value(
    arguments: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> Result<String, String> {
    arguments
        .next()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| format!("{flag} needs a value"))
}

fn write_result(prefix: &Path, result: &RunResult) -> std::io::Result<()> {
    fs::write(
        path_with_suffix(prefix, ".action"),
        match result.reason {
            ExitReason::Accepted => "accept",
            ExitReason::Cancelled => "cancel",
            ExitReason::Interrupted => "interrupt",
        },
    )?;
    fs::write(path_with_suffix(prefix, ".buffer"), &result.buffer)?;
    fs::write(
        path_with_suffix(prefix, ".cursor"),
        result.cursor.to_string(),
    )?;
    Ok(())
}

fn path_with_suffix(prefix: &Path, suffix: &str) -> PathBuf {
    let mut path = prefix.as_os_str().to_os_string();
    path.push(suffix);
    PathBuf::from(path)
}

fn print_usage() {
    println!(
        "keel-demo\n\nA Rust-owned line editor demo for stock Zsh.\n\nUsage:\n  keel-demo [--buffer TEXT] [--cursor N] [--prompt TEXT]\n            [--result-prefix PATH]\n\nThe result-prefix form writes .action, .buffer, and .cursor files for the\nsmall Zsh widget bridge."
    );
}
