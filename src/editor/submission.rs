mod probe;
use std::{io, sync::mpsc, thread};

#[derive(Default)]
pub(crate) struct Checker {
    worker: Option<Worker>,
    generation: u64,
    pending: Option<(u64, String)>,
}

struct Worker {
    requests: mpsc::Sender<(u64, String)>,
    results: mpsc::Receiver<(u64, io::Result<bool>)>,
}

impl Checker {
    pub(crate) fn request(&mut self, buffer: &str) -> io::Result<()> {
        if self.worker.is_none() {
            self.worker = Some(Worker::start()?);
        }
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some((self.generation, buffer.into()));
        if let Some(worker) = &self.worker {
            worker
                .requests
                .send((self.generation, buffer.into()))
                .map_err(|_| io::Error::other("Zsh syntax worker stopped"))?;
        }
        Ok(())
    }

    pub(crate) fn pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn cancel(&mut self) {
        self.pending = None;
    }

    pub(crate) fn poll(&mut self, buffer: &str) -> io::Result<Option<bool>> {
        let Some(worker) = &self.worker else {
            return Ok(None);
        };
        while let Ok((id, result)) = worker.results.try_recv() {
            if self
                .pending
                .as_ref()
                .is_some_and(|(generation, code)| *generation == id && code == buffer)
            {
                self.pending = None;
                return result.map(Some);
            }
        }
        Ok(None)
    }
}

impl Worker {
    fn start() -> io::Result<Self> {
        let (requests, input) = mpsc::channel::<(u64, String)>();
        let (output, results) = mpsc::channel();
        thread::Builder::new()
            .name("shell-syntax".into())
            .spawn(move || {
                let mut parser = probe::Parser::default();
                while let Ok((id, buffer)) = input.recv() {
                    let result = parser.check(&buffer);
                    if result.is_err() {
                        parser = probe::Parser::default();
                    }
                    if output.send((id, result)).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self { requests, results })
    }
}

#[cfg(test)]
mod tests;
