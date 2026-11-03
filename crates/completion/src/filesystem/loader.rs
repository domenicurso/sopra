use super::cache::Entry;
use notify::{RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, SyncSender},
    },
};

pub(super) struct Loader {
    pub(super) requests: SyncSender<PathBuf>,
    pub(super) replies: Receiver<Reply>,
}

pub(super) enum Reply {
    Loaded(PathBuf, Vec<Entry>),
    Invalidated(PathBuf),
}

impl Loader {
    pub(super) fn spawn() -> Option<Self> {
        let (requests, pending) = mpsc::sync_channel::<PathBuf>(16);
        let (responses, replies) = mpsc::channel();
        // Filesystem calls cannot be cancelled portably. Never join this thread on the editor path.
        std::thread::Builder::new()
            .name("completion-files".into())
            .spawn(move || {
                let events = responses.clone();
                let roots = Arc::new(WatchRoots::default());
                let observed = roots.clone();
                let mut watcher = notify::recommended_watcher(
                    move |event: Result<notify::Event, notify::Error>| {
                        if let Ok(event) = event {
                            for path in observed.affected(&event.paths) {
                                let _ = events.send(Reply::Invalidated(path));
                            }
                        }
                    },
                )
                .ok();
                while let Ok(path) = pending.recv() {
                    if roots.track(&path)
                        && let Some(watcher) = watcher.as_mut()
                    {
                        let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
                    }
                    let entries = read(&path);
                    if responses.send(Reply::Loaded(path, entries)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Self { requests, replies })
    }
}

#[derive(Default)]
struct WatchRoots {
    aliases: Mutex<HashMap<PathBuf, Vec<PathBuf>>>,
}

impl WatchRoots {
    fn track(&self, path: &Path) -> bool {
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.into());
        let Ok(mut aliases) = self.aliases.lock() else {
            return false;
        };
        let paths = aliases.entry(canonical).or_default();
        let first = paths.is_empty();
        if !paths.iter().any(|tracked| tracked == path) {
            paths.push(path.into());
        }
        first
    }

    fn affected(&self, changes: &[PathBuf]) -> Vec<PathBuf> {
        let Ok(aliases) = self.aliases.lock() else {
            return Vec::new();
        };
        // macOS reports canonical /private paths even when a watcher was registered through /var.
        aliases
            .iter()
            .filter(|(root, _)| changes.iter().any(|path| path.starts_with(root)))
            .flat_map(|(_, paths)| paths.iter().cloned())
            .collect()
    }
}

pub(super) fn read(directory: &Path) -> Vec<Entry> {
    fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let metadata = entry.metadata().ok()?;
            Some(Entry {
                name,
                path: entry.path(),
                directory: metadata.is_dir(),
                modified: metadata.modified().ok(),
            })
        })
        .collect()
}
