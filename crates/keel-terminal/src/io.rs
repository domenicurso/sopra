use std::{io, time::Duration};

use keel_core::{CanvasBuffer, InputEvent, TerminalSize};
use keel_render::CanvasPatch;

pub trait TerminalIo {
    fn size(&self) -> io::Result<TerminalSize>;
    fn enter_frontend(&mut self) -> io::Result<()>;
    fn read_event(&mut self, timeout: Duration) -> io::Result<Option<InputEvent>>;
    fn draw(&mut self, patch: &CanvasPatch) -> io::Result<()>;
    fn finalize_submission(&mut self, canvas: &CanvasBuffer, patch: &CanvasPatch) -> io::Result<()>;
    fn suspend_for_command(&mut self) -> io::Result<()>;
    fn resume_frontend(&mut self, canvas: &CanvasBuffer) -> io::Result<()>;
    fn clear_frontend(&mut self, canvas: &CanvasBuffer) -> io::Result<()>;
}
