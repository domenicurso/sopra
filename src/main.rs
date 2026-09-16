mod animation;
mod completion;
mod editor;
mod input;
mod palette;
mod render;
mod scene;
mod syntax;

use std::{env, error::Error, ffi::OsString, path::PathBuf};

use editor::{EditorConfig, EditorState, ExitReason, RunResult};
use input::{CursorPosition, Terminal};

#[derive(Debug)]
struct Args {
    buffer: String,
    cursor: usize,
    prompt: String,
    cwd: PathBuf,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut buffer = String::new();
        let mut cursor = 0;
        let mut prompt = "keel ❯ ".to_string();
        let mut cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
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
                "--cwd" => cwd = PathBuf::from(next_value(&mut arguments, "--cwd")?),
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
            cwd,
        })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse().map_err(|message| format!("keel: {message}"))?;
    let mut terminal = Terminal::open()?;
    let size = terminal.size()?;
    // ZLE has already positioned the terminal on the active line. The renderer saves that
    // position and treats it as row zero, so Keel does not need terminal cursor queries.
    let anchor = CursorPosition { row: 0, column: 0 };
    let mut editor = EditorState::new(EditorConfig {
        buffer: args.buffer,
        cursor_chars: args.cursor,
        prompt: args.prompt,
        anchor,
        size,
        cwd: args.cwd,
        palette: terminal.palette(),
    });
    let result = editor.run(&mut terminal);

    if result.is_err() {
        let _ = terminal.write_all(b"\x1b[0m\x1b[?25h");
        let _ = terminal.flush();
    }
    terminal.restore()?;

    let result = result?;
    write_result(&result)?;
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

fn write_result(result: &RunResult) -> std::io::Result<()> {
    use std::io::Write;

    let action = match result.reason {
        ExitReason::Accepted => "accept",
        ExitReason::Cancelled => "cancel",
        ExitReason::Interrupted => "interrupt",
        ExitReason::DelegateUp => "up",
        ExitReason::DelegateDown => "down",
        ExitReason::DelegateTab => "tab",
        ExitReason::DelegateEof => "eof",
    };
    let mut stdout = std::io::stdout().lock();
    writeln!(
        stdout,
        "K1\t{action}\t{}\t{}",
        result.cursor,
        hex_encode(result.buffer.as_bytes())
    )
}

fn print_usage() {
    println!(
        "keel\n\nA Rust-owned line editor for stock Zsh.\n\nUsage:\n  keel [--buffer TEXT] [--cursor N] [--prompt TEXT] [--cwd PATH]"
    );
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
