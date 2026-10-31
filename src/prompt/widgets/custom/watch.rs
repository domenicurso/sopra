use super::{compiler, process::Process};
use std::{fs, path::Path};

#[derive(Default)]
enum Source {
    #[default]
    Unread,
    Missing,
    Contents(Vec<u8>),
}

#[derive(Default)]
pub(super) struct Watch {
    previous: Source,
    pub process: Option<Process>,
}

impl Watch {
    pub(super) fn refresh(&mut self, source: &Path) -> std::io::Result<bool> {
        let bytes = match fs::read(source) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.process = None;
                if matches!(self.previous, Source::Missing) {
                    return Ok(false);
                }
                self.previous = Source::Missing;
                return Err(error);
            }
        };
        if matches!(&self.previous, Source::Contents(previous) if *previous == bytes) {
            return Ok(false);
        }
        let result = compiler::build(source, &bytes).and_then(|binary| Process::start(&binary));
        self.previous = Source::Contents(bytes);
        self.process = Some(result?);
        Ok(true)
    }
}
