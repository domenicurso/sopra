mod actions;
mod blocks;
mod bootstrap;
mod browser;
mod callbacks;
mod cells;
mod files;
mod input;
mod interaction;
mod layout;
mod model;
mod mouse;
mod opener;
mod paint;
mod protocol;
mod pty;
mod rows;
mod search;
mod selection;
mod signals;
mod terminal;
#[cfg(test)]
mod transcript_tests;
mod transport;
mod view;

use std::{
    io,
    time::{Duration, Instant},
};

pub(crate) fn run() -> io::Result<u32> {
    let config = crate::config::Config::read()?;
    let _signals = signals::Signals::install()?;
    let terminal = terminal::Terminal::open()?;
    let size = terminal.size()?;
    let shell = pty::Shell::spawn(size)?;
    Session {
        terminal,
        shell,
        model: model::Model::new(size, config.session),
        browser: None,
        input: input::Input::default(),
        painter: paint::Painter::default(),
        pasted: false,
        snapshot: Instant::now(),
        openers: opener::Openers::default(),
    }
    .run()
}

struct Session {
    shell: pty::Shell,
    terminal: terminal::Terminal,
    model: model::Model,
    browser: Option<browser::Browser>,
    input: input::Input,
    painter: paint::Painter,
    pasted: bool,
    snapshot: Instant,
    openers: opener::Openers,
}
impl Session {
    fn run(mut self) -> io::Result<u32> {
        loop {
            if let Some(signal) = signals::Signals::received() {
                return Ok(128 + signal as u32);
            }
            let before = self.model.revision;
            let mut changed = self.output()?;
            changed |= self.resize()?;
            changed |= self.refresh_history(before)?;
            self.shell.writer.flush()?;
            let input = if self.shell.writer.ready() {
                self.terminal.read()?
            } else {
                std::thread::sleep(Duration::from_millis(8));
                Vec::new()
            };
            for packet in self.input.feed(&input) {
                self.input_packet(&packet)?;
                changed = true;
            }
            if changed {
                self.painter
                    .render(&self.model, self.browser.as_ref(), &mut self.terminal)?;
            }
            if let Some(status) = self.shell.child.try_wait()? {
                self.output()?;
                return Ok(status.exit_code());
            }
        }
    }
    fn refresh_history(&mut self, before: u64) -> io::Result<bool> {
        let mut changed = false;
        for notice in self.openers.poll() {
            if let Some(browser) = &mut self.browser {
                browser.notice = notice;
                changed = true;
            }
        }
        if self.browser.is_some()
            && self.model.executing
            && self.snapshot.elapsed() > Duration::from_millis(150)
        {
            self.model.blocks.snapshot()?;
            self.invalidate_live_selection();
            self.snapshot = Instant::now();
            self.refresh_browser();
            changed = true;
        }
        if before != self.model.revision {
            self.invalidate_live_selection();
            self.refresh_browser();
        }
        Ok(changed)
    }

    fn output(&mut self) -> io::Result<bool> {
        let mut changed = false;
        for _ in 0..16 {
            let Ok(bytes) = self.shell.output.try_recv() else {
                break;
            };
            self.model.process(&bytes?)?;
            let callbacks = self.model.terminal.callbacks_mut();
            self.shell.send(&std::mem::take(&mut callbacks.replies))?;
            self.terminal
                .write(&std::mem::take(&mut callbacks.terminal))?;
            changed = true;
        }
        Ok(changed)
    }
    fn resize(&mut self) -> io::Result<bool> {
        let size = self.terminal.size()?;
        if (size.rows, size.cols) == (self.model.size.rows, self.model.size.cols) {
            return Ok(false);
        }
        self.model.resize(size);
        self.shell.master.resize(size).map_err(io::Error::other)?;
        self.refresh_browser();
        Ok(true)
    }
    fn invalidate_live_selection(&mut self) {
        if let (Some(browser), Some(block)) = (&mut self.browser, self.model.blocks.entries.back())
        {
            browser.selection.invalidate(block.id);
        }
    }
    fn refresh_browser(&mut self) {
        if let Some(browser) = &mut self.browser {
            browser.refresh(&self.model.blocks, self.model.size.cols);
        }
    }
    fn browse(&mut self) -> io::Result<()> {
        if self.browser.is_none() {
            self.model.blocks.snapshot()?;
            self.browser = Some(browser::Browser::new(
                &self.model.blocks,
                self.model.size.cols,
                layout::Layout::new(&self.model).height,
            ));
        }
        Ok(())
    }
}
