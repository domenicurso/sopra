mod codec;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    time::Duration,
};

use super::discover::HelpRequest;

const CACHE_LIMIT: u64 = 1_000_000;
pub(crate) const MAX_AGE: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Source {
    Help(String),
    Man(String),
}

pub(super) fn read(request: &HelpRequest) -> Option<Vec<Source>> {
    let path = cache_path(request);
    let metadata = fs::metadata(&path).ok()?;
    if metadata.len() > CACHE_LIMIT || metadata.modified().ok()?.elapsed().ok()? > MAX_AGE {
        return None;
    }
    codec::decode(&fs::read(path).ok()?)
}

pub(super) fn write(request: &HelpRequest, sources: &[Source]) {
    let path = cache_path(request);
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    if let Some(bytes) = codec::encode(sources) {
        let _ = replace(&path, &bytes);
    }
}

fn cache_path(request: &HelpRequest) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    collection_fingerprint().hash(&mut hasher);
    request.program.hash(&mut hasher);
    request.args.hash(&mut hasher);
    let resolved = request
        .command
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| format!("builtin:{}", request.program));
    super::identity::key(&resolved, request.command.as_deref(), &request.cwd).hash(&mut hasher);
    crate::cache::command_directory("completion-cache", &request.program)
        .join(format!("{:016x}.cache", hasher.finish()))
}

pub(super) fn inspect(request: &HelpRequest) -> String {
    use std::fmt::Write;
    let path = cache_path(request);
    let mut output = format!(
        "Cache file: {}\nCollector fingerprint: {:016x}\n",
        path.display(),
        collection_fingerprint()
    );
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            let _ = writeln!(output, "State: unavailable ({error})");
            return output;
        }
    };
    let age = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.elapsed().ok());
    let _ = writeln!(
        output,
        "Bytes: {}\nAge: {} seconds",
        metadata.len(),
        age.map_or_else(|| "unknown".into(), |age| age.as_secs().to_string())
    );
    if metadata.len() > CACHE_LIMIT {
        let _ = writeln!(output, "State: oversized bundle");
        return output;
    }
    let sources = fs::read(path).ok().and_then(|bytes| codec::decode(&bytes));
    let state = if sources.is_none() {
        "invalid binary bundle (checksum or framing)"
    } else if age.is_none_or(|age| age > MAX_AGE) {
        "expired"
    } else {
        "valid"
    };
    let _ = writeln!(output, "State: {state}");
    for source in sources.into_iter().flatten() {
        let (kind, text) = match source {
            Source::Help(text) => ("help", text),
            Source::Man(text) => ("man", text),
        };
        let _ = writeln!(output, "Source: {kind}, {} bytes", text.len());
    }
    output
}

// A changed collector or binary codec automatically retires old bundles; parser-only changes reparse them.
fn collection_fingerprint() -> u64 {
    static FINGERPRINT: std::sync::LazyLock<u64> = std::sync::LazyLock::new(|| {
        let mut hash = DefaultHasher::new();
        for source in [
            include_bytes!("probes.rs").as_slice(),
            include_bytes!("extra_help.rs").as_slice(),
            include_bytes!("process.rs").as_slice(),
            include_bytes!("normalize.rs").as_slice(),
            include_bytes!("cache/codec.rs").as_slice(),
        ] {
            source.hash(&mut hash);
        }
        hash.finish()
    });
    *FINGERPRINT
}

// Readers see a complete source bundle even when workers discover the same command together.
pub(super) fn replace(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::{
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let temporary = path.with_extension(format!(
        "{}.{}.pending",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_a_bundle_never_appends_to_the_previous_sources() -> std::io::Result<()> {
        let path = std::env::temp_dir().join(format!("sopra-cache-test-{}", std::process::id()));
        replace(&path, b"long previous source")?;
        replace(&path, b"new")?;
        assert_eq!(fs::read(&path)?, b"new");
        fs::remove_file(path)
    }
}
