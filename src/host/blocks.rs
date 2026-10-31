use crate::config::SessionConfig;
use std::{
    collections::VecDeque,
    io,
    path::PathBuf,
    time::{Duration, Instant},
};
use vt100::capture::{BasicFormattedCaptureRange, basic_formatted_to_plain};

pub(super) struct Block {
    pub(super) id: u64,
    pub(super) command: String,
    pub(super) cwd: PathBuf,
    pub(super) output: String,
    pub(super) status: Option<i32>,
    pub(super) duration: Duration,
    pub(super) collapsed: bool,
    started: Instant,
}
pub(super) struct Blocks {
    pub(super) entries: VecDeque<Block>,
    capture: Option<vt100::Parser>,
    captured: usize,
    truncated: bool,
    limits: SessionConfig,
    next: u64,
}
impl Blocks {
    pub(super) fn new(limits: SessionConfig) -> Self {
        Self {
            entries: VecDeque::new(),
            capture: None,
            captured: 0,
            truncated: false,
            limits,
            next: 1,
        }
    }
    pub(super) fn begin(&mut self, command: String, cwd: PathBuf, size: portable_pty::PtySize) {
        self.next += 1;
        self.entries.push_back(Block {
            id: self.next - 1,
            command,
            cwd,
            output: String::new(),
            status: None,
            duration: Duration::ZERO,
            collapsed: false,
            started: Instant::now(),
        });
        self.capture = Some(super::model::parser(size, self.limits.scrollback_lines));
        self.captured = 0;
        self.truncated = false;
        self.trim();
    }
    pub(super) fn process(&mut self, bytes: &[u8]) {
        if let Some(capture) = &mut self.capture {
            let remaining = self.limits.max_output_bytes.saturating_sub(self.captured);
            capture.process(&bytes[..bytes.len().min(remaining)]);
            self.truncated |= bytes.len() > remaining;
            self.captured = self.captured.saturating_add(bytes.len().min(remaining));
        }
    }
    pub(super) fn finish(&mut self, status: i32) -> io::Result<()> {
        if self.capture.is_none() {
            return Ok(());
        }
        self.snapshot()?;
        self.capture = None;
        if let Some(block) = self.entries.back_mut() {
            block.status = Some(status);
            block.duration = block.started.elapsed();
        }
        self.trim();
        Ok(())
    }
    pub(super) fn snapshot(&mut self) -> io::Result<()> {
        let (Some(capture), Some(block)) = (&self.capture, self.entries.back_mut()) else {
            return Ok(());
        };
        let mut formatted = String::new();
        capture
            .screen()
            .write_contents_formatted_basic(
                &mut formatted,
                BasicFormattedCaptureRange::Full(&mut Default::default()),
            )
            .map_err(io::Error::other)?;
        block.output = basic_formatted_to_plain(&formatted)
            .collect::<String>()
            .trim_end_matches('\n')
            .to_string();
        if self.truncated {
            block
                .output
                .push_str("\n[output truncated at session.max_output_bytes]");
        }
        block.duration = block.started.elapsed();
        Ok(())
    }
    pub(super) fn get(&self, id: u64) -> Option<&Block> {
        self.entries.iter().find(|block| block.id == id)
    }
    pub(super) fn toggle(&mut self, id: u64) {
        if let Some(block) = self.entries.iter_mut().find(|block| block.id == id) {
            block.collapsed = !block.collapsed;
        }
    }
    fn trim(&mut self) {
        let mut bytes: usize = self
            .entries
            .iter()
            .map(|block| block.command.len() + block.output.len())
            .sum();
        while self.entries.len() > 1
            && (self.entries.len() > self.limits.max_blocks || bytes > self.limits.max_output_bytes)
        {
            if let Some(block) = self.entries.pop_front() {
                bytes = bytes.saturating_sub(block.command.len() + block.output.len());
            }
        }
    }
}
