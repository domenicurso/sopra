use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Clone)]
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) path: PathBuf,
    pub(super) directory: bool,
}

pub(super) struct DirectoryCache {
    entries: HashMap<PathBuf, Vec<Entry>>,
    invalidations: Receiver<PathBuf>,
    watcher: Option<RecommendedWatcher>,
}

impl DirectoryCache {
    pub(super) fn new() -> Self {
        let (sender, invalidations) = mpsc::channel();
        let watcher = notify::recommended_watcher(move |event: Result<Event, notify::Error>| {
            if let Ok(event) = event {
                for path in event.paths {
                    let _ = sender.send(path);
                }
            }
        })
        .ok();
        Self {
            entries: HashMap::new(),
            invalidations,
            watcher,
        }
    }

    pub(super) fn entries(&mut self, directory: &Path) -> Vec<Entry> {
        self.drain_invalidations();
        if let Some(entries) = self.entries.get(directory) {
            return entries.clone();
        }
        let entries = fs::read_dir(directory)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let directory = fs::metadata(entry.path()).ok()?.is_dir();
                Some(Entry {
                    name,
                    path: entry.path(),
                    directory,
                })
            })
            .collect::<Vec<_>>();
        if let Some(watcher) = self.watcher.as_mut() {
            let _ = watcher.watch(directory, RecursiveMode::NonRecursive);
        }
        self.entries
            .insert(directory.to_path_buf(), entries.clone());
        entries
    }

    fn drain_invalidations(&mut self) {
        while let Ok(path) = self.invalidations.try_recv() {
            self.entries.remove(&path);
            if let Some(parent) = path.parent() {
                self.entries.remove(parent);
            }
        }
    }
}
