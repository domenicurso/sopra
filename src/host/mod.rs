mod bootstrap;
mod callbacks;
mod cells;
mod input;
mod interaction;
mod model;
mod paint;
mod pty;
mod signals;
mod terminal;
mod transport;

use std::{io, time::Duration};

pub(crate) fn run() -> io::Result<u32> {
    let config = crate::config::Config::read()?;
    let _signals = signals::Signals::install()?;
    let terminal = terminal::Terminal::open()?;
    let size = terminal.size()?;
    let shell = pty::Shell::spawn(size)?;
    Session {
        terminal,
        shell,
        model: model::Model::new(size, config.session.scrollback_lines),
        input: input::Input::default(),
        painter: paint::Painter::default(),
        pasted: false,
    }
    .run()
}

struct Session {
    shell: pty::Shell,
    terminal: terminal::Terminal,
    model: model::Model,
    input: input::Input,
    painter: paint::Painter,
    pasted: bool,
}
impl Session {
    fn run(mut self) -> io::Result<u32> {
        loop {
            if let Some(signal) = signals::Signals::received() {
                return Ok(128 + signal as u32);
            }
            let mut changed = self.output()?;
            changed |= self.resize()?;
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
                self.painter.render(&self.model, &mut self.terminal)?;
            }
            if let Some(status) = self.shell.child.try_wait()? {
                self.output()?;
                return Ok(status.exit_code());
            }
        }
    }
    fn output(&mut self) -> io::Result<bool> {
        let mut changed = false;
        for _ in 0..16 {
            let Ok(bytes) = self.shell.output.try_recv() else {
                break;
            };
            self.model.process(&bytes?);
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
        Ok(true)
    }
}
