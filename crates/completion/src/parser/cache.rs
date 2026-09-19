use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    time::UNIX_EPOCH,
};

use super::discover::HelpRequest;

const CACHE_LIMIT: u64 = 1_000_000;
const CACHE_MARKER: &str = "\n\x1eSOPRA_COMPLETION_SOURCE\x1e\n";
const CACHE_VERSION: &str = "SOPRA_COMPLETION_CACHE_V4";

#[derive(Debug)]
pub(super) enum Source {
    Help(String),
    Man(String),
}

pub(super) fn read(request: &HelpRequest) -> Option<Vec<Source>> {
    let path = cache_path(request)?;
    let metadata = fs::metadata(&path).ok()?;
    if metadata.len() > CACHE_LIMIT {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    let mut parts = text.split(CACHE_MARKER);
    if parts.next()? != CACHE_VERSION {
        return None;
    }
    Some(parts.filter_map(source).collect())
}

pub(super) fn write(request: &HelpRequest, sources: &[Source]) {
    let Some(path) = cache_path(request) else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let mut text = String::from(CACHE_VERSION);
    for source in sources {
        let (kind, value) = match source {
            Source::Help(value) => ("help", value),
            Source::Man(value) => ("man", value),
        };
        text.push_str(CACHE_MARKER);
        text.push_str(kind);
        text.push('\n');
        text.push_str(value);
    }
    let _ = fs::write(path, text);
}

fn source(part: &str) -> Option<Source> {
    part.strip_prefix("help\n")
        .map(|value| Source::Help(value.to_string()))
        .or_else(|| {
            part.strip_prefix("man\n")
                .map(|value| Source::Man(value.to_string()))
        })
}

fn cache_path(request: &HelpRequest) -> Option<PathBuf> {
    let mut hasher = DefaultHasher::new();
    request.program.hash(&mut hasher);
    request.args.hash(&mut hasher);
    request.cwd.hash(&mut hasher);
    if let Some(command) = &request.command {
        command.hash(&mut hasher);
        let metadata = fs::metadata(command).ok()?;
        metadata.len().hash(&mut hasher);
        metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos()
            .hash(&mut hasher);
    }
    Some(
        std::env::temp_dir()
            .join("sopra")
            .join("completion-cache")
            .join(format!("{:016x}.txt", hasher.finish())),
    )
}
