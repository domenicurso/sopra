mod animation;
mod arguments;
mod editor;
mod input;
mod palette;
mod prompt;
mod render;
mod scene;
mod syntax;

use std::error::Error;

use arguments::Args;
use editor::{EditorConfig, EditorState, ExitReason, RunResult};
use input::{CursorPosition, Terminal};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() == Some(std::ffi::OsStr::new("--prompt-config")) {
        let path = arguments.next().ok_or("--prompt-config needs a path")?;
        prompt::config::PromptConfig::read(std::path::Path::new(&path))?.print();
        return Ok(());
    }
    let args = Args::parse().map_err(|message| format!("sopra: {message}"))?;
    let mut terminal = Terminal::open()?;
    let size = terminal.size()?;
    // ZLE has already positioned the terminal on the active line. The renderer treats that
    // position as logical row zero and resolves its physical row when a resize needs it.
    let anchor = CursorPosition { row: 0, column: 0 };
    let mut editor = EditorState::new(EditorConfig {
        buffer: args.buffer,
        cursor_chars: args.cursor,
        arm_completion: args.arm_completion,
        suppress_completion: args.suppress_completion,
        completion_enabled: args.completion_enabled,
        syntax_highlighting: args.syntax_highlighting,
        cursor_animation: args.cursor_animation,
        prompt: args.prompt,
        rprompt: args.rprompt,
        anchor,
        size,
        cwd: args.cwd,
        palette: terminal.palette(),
    });
    if let Ok(prompt) = std::env::var("SOPRA_EXPANDED_TRANSIENT_PROMPT") {
        editor.set_transient_prompt(prompt);
    }
    let result = editor.run(&mut terminal);

    if result.is_err() {
        let _ = terminal.write_all(b"\x1b[0m\x1b[?7h\x1b[?2026l\x1b[?25h");
        let _ = terminal.flush();
    }
    terminal.restore()?;

    let result = result?;
    write_result(&result)?;
    Ok(())
}

fn write_result(result: &RunResult) -> std::io::Result<()> {
    use std::io::Write;

    let action = match result.reason {
        ExitReason::Accepted => "accept",
        ExitReason::Interrupted => "interrupt",
        ExitReason::DelegateUp => "up",
        ExitReason::DelegateDown => "down",
        ExitReason::DelegateTab => "tab",
        ExitReason::DelegateEof => "eof",
        ExitReason::ChangeDirectory => "directory",
    };
    let mut stdout = std::io::stdout().lock();
    writeln!(
        stdout,
        "K1\t{action}\t{}\t{}\t{}\t{}",
        result.cursor,
        nonempty_hex(result.buffer.as_bytes()),
        match encode_highlights(&result.buffer, &result.highlights) {
            value if value.is_empty() => "-".to_string(),
            value => value,
        },
        result.directory.as_ref().map_or_else(
            || "-".to_string(),
            |path| {
                use std::os::unix::ffi::OsStrExt;
                nonempty_hex(path.as_os_str().as_bytes())
            }
        )
    )
}

fn encode_highlights(buffer: &str, spans: &[crate::syntax::SyntaxSpan]) -> String {
    spans
        .iter()
        .filter_map(|span| {
            let style = crate::render::region_highlight_style(span.style())?;
            let start = buffer[..span.start].chars().count();
            let end = buffer[..span.end].chars().count();
            (start < end).then(|| format!("{start} {end} {style}"))
        })
        .collect::<Vec<_>>()
        .join(";")
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

fn nonempty_hex(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        "-".to_string()
    } else {
        hex_encode(bytes)
    }
}
