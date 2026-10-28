mod animation;
mod arguments;
mod editor;
mod input;
mod palette;
mod prompt;
mod protocol;
mod render;
mod scene;
mod syntax;

use std::error::Error;

use arguments::Args;
use editor::{EditorConfig, EditorState};
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
    let mut editor = editor_for(args, &terminal)?;
    if let Ok(prompt) = std::env::var("SOPRA_EXPANDED_TRANSIENT_PROMPT") {
        editor.set_transient_prompt(prompt);
    }
    if let Ok(prompt) = std::env::var("SOPRA_EXPANDED_SECONDARY_PROMPT") {
        editor.set_secondary_prompt(prompt);
    }
    let result = editor.run(&mut terminal);

    if result.is_err() {
        let _ = terminal.write_all(b"\x1b[0m\x1b[?7h\x1b[?2026l\x1b[?25h");
        let _ = terminal.flush();
    }
    terminal.restore()?;

    let result = result?;
    protocol::write_result(&result)?;
    Ok(())
}

fn editor_for(args: Args, terminal: &Terminal) -> std::io::Result<EditorState> {
    Ok(EditorState::new(EditorConfig {
        buffer: args.buffer,
        cursor_chars: args.cursor,
        arm_completion: args.arm_completion,
        suppress_completion: args.suppress_completion,
        completion_enabled: args.completion_enabled,
        syntax_highlighting: args.syntax_highlighting,
        cursor_animation: args.cursor_animation,
        prompt: args.prompt,
        rprompt: args.rprompt,
        anchor: CursorPosition { row: 0, column: 0 },
        size: terminal.size()?,
        cwd: args.cwd,
        palette: terminal.palette(),
    }))
}
