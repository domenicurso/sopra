use crate::{
    arguments::Args,
    editor::{self, EditorConfig, EditorState, RunResult},
    input::{CursorPosition, Terminal},
    palette::TerminalPalette,
};
use std::error::Error;

pub(super) fn run(
    args: Args,
    palette: &mut Option<TerminalPalette>,
) -> Result<RunResult, Box<dyn Error>> {
    let mut terminal = Terminal::open(*palette)?;
    *palette = Some(terminal.palette());
    let history = editor::History::read(args.history_file.as_deref())?;
    let mut editor = editor_for(args, &terminal)?;
    editor.set_history(history);
    if let Ok(prompt) = sopra_shell_context::var("SOPRA_EXPANDED_TRANSIENT_PROMPT") {
        editor.set_transient_prompt(prompt);
    }
    if let Ok(prompt) = sopra_shell_context::var("SOPRA_EXPANDED_SECONDARY_PROMPT") {
        editor.set_secondary_prompt(prompt);
    }
    let result = editor.run(&mut terminal);

    if result.is_err() {
        let _ = terminal.write_all(b"\x1b[0m\x1b[?7h\x1b[?2026l\x1b[?25h");
        let _ = terminal.flush();
    }
    terminal.restore()?;

    let result = result?;
    Ok(result)
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
