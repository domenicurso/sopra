use super::{blocks::Blocks, callbacks::Callbacks, protocol::Event};
use crate::config::SessionConfig;
use portable_pty::PtySize;
use std::{io, num::NonZeroU16};

pub(super) struct Model {
    pub(super) terminal: vt100::Parser<Callbacks>,
    pub(super) blocks: Blocks,
    pub(super) prompt: Option<u16>,
    pub(super) executing: bool,
    pub(super) secondary: bool,
    pub(super) size: PtySize,
    pub(super) revision: u64,
}
impl Model {
    pub(super) fn new(size: PtySize, limits: SessionConfig) -> Self {
        let (rows, cols) = dimensions(size);
        Self {
            terminal: vt100::Parser::new_with_callbacks(
                rows,
                cols,
                limits.scrollback_lines,
                Callbacks::default(),
            ),
            blocks: Blocks::new(limits),
            prompt: None,
            executing: false,
            secondary: false,
            size,
            revision: 0,
        }
    }
    pub(super) fn process(&mut self, bytes: &[u8]) -> io::Result<()> {
        // OSC terminators are transaction boundaries: command output preceding an end marker
        // must be captured before the next prompt, even if both arrive in one PTY read.
        for chunk in bytes.split_inclusive(|byte| *byte == 7 || *byte == b'\\') {
            self.blocks.process(chunk);
            self.terminal.process(chunk);
            for event in std::mem::take(&mut self.terminal.callbacks_mut().events) {
                match event {
                    Event::Executing => {
                        self.executing = true;
                        self.prompt = None;
                    }
                    Event::Begin { command, cwd } => {
                        self.revision += 1;
                        self.executing = true;
                        self.prompt = None;
                        self.blocks.begin(command, cwd, self.size);
                    }
                    Event::End(status) => {
                        self.revision += 1;
                        self.blocks.finish(status)?;
                        self.executing = false;
                    }
                    Event::Editing { secondary } => self.secondary = secondary,
                    Event::Prompt(row) => self.prompt = Some(row),
                }
            }
        }
        Ok(())
    }
    pub(super) fn resize(&mut self, size: PtySize) {
        let (rows, cols) = dimensions(size);
        self.terminal.set_size(rows, cols);
        self.size = size;
        self.prompt = self.prompt.map(|row| row.min(size.rows.saturating_sub(1)));
    }
}
pub(super) fn parser(size: PtySize, scrollback: usize) -> vt100::Parser {
    let (rows, cols) = dimensions(size);
    vt100::Parser::new(rows, cols, scrollback)
}
fn dimensions(size: PtySize) -> (NonZeroU16, NonZeroU16) {
    (
        NonZeroU16::new(size.rows).unwrap_or(NonZeroU16::MIN),
        NonZeroU16::new(size.cols).unwrap_or(NonZeroU16::MIN),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_capture_progress_and_unicode_without_prompt_redraws() -> io::Result<()> {
        let limits = SessionConfig {
            max_blocks: 2,
            scrollback_lines: 100,
            max_output_bytes: 4096,
        };
        let mut model = Model::new(PtySize::default(), limits);
        let stream = b"\x1b]777;sopra;begin;6563686f;2f746d70\x07old\r\x1b[Knew\r\n\x1b]777;sopra;end;7\x07prompt";
        for byte in stream {
            model.process(&[*byte])?;
        }
        let block = &model.blocks.entries[0];
        assert_eq!(block.command, "echo");
        assert_eq!(block.output, "new");
        assert_eq!(block.status, Some(7));
        assert!(!model.executing);
        Ok(())
    }
}
