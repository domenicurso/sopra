use std::{io, time::Duration};

use keel_core::{InputEvent, TerminalSize};
use keel_render::FramePatch;

pub trait TerminalIo {
    fn size(&self) -> io::Result<TerminalSize>;
    fn read_event(&mut self, timeout: Duration) -> io::Result<Option<InputEvent>>;
    fn apply_patch(&mut self, patch: &FramePatch) -> io::Result<()>;
    fn clear_render_region(&mut self, previous_height: u16) -> io::Result<()>;
}
