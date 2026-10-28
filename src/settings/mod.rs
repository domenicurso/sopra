mod config;
mod controls;
mod drawing;
mod migration;
mod panel;
mod schema;
mod state;
mod view;

use std::{io, time::Duration};

use crate::{
    input::{Terminal, TerminalSize},
    palette::TerminalPalette,
    render::Renderer,
};

use state::SettingsState;

pub(crate) fn run() -> io::Result<()> {
    let mut terminal = Terminal::open()?;
    let palette = terminal.palette();
    let result = run_overlay(&mut terminal, palette);
    let restore = terminal.restore();
    result.and(restore)
}

fn run_overlay(terminal: &mut Terminal, palette: TerminalPalette) -> io::Result<()> {
    terminal.write_all(b"\x1b[H")?;
    terminal.flush()?;
    let size = terminal.size()?;
    let mut state = SettingsState::load();
    let result = run_loop(&mut state, terminal, palette, size);
    if result.is_err() {
        let _ = terminal.write_all(b"\x1b[0m\x1b[?7h\x1b[?2026l\x1b[?25h");
        let _ = terminal.flush();
    }
    result
}

fn run_loop(
    state: &mut SettingsState,
    terminal: &mut Terminal,
    palette: TerminalPalette,
    initial_size: TerminalSize,
) -> io::Result<()> {
    let mut renderer = Renderer::overlay();
    let mut size = initial_size;
    loop {
        renderer.render(view::build(state, size, palette), terminal)?;
        if terminal.poll(Duration::from_millis(100))? && state.handle_key(terminal.read_key()?) {
            return renderer.finish(
                terminal,
                crate::input::CursorPosition { column: 0, row: 0 },
                None,
            );
        }
        size = terminal.size().unwrap_or(size);
    }
}
