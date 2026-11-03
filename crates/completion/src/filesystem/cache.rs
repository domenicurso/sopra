use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::SystemTime,
};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Clone)]
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) path: PathBuf,
    pub(super) directory: bool,
    pub(super) modified: Option<SystemTime>,
}

pub(super) struct DirectoryCache {
    entries: HashMap<PathBuf, Vec<Entry>>,
    invalidations: Receiver<PathBuf>,
    watcher: Option<RecommendedWatcher>,
    loading: Loading,
    pending: HashSet<PathBuf>,
}

enum Loading {
    Blocking,
    Background(Option<super::loader::Loader>),
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
            loading: Loading::Blocking,
            pending: HashSet::new(),
        }
    }

    pub(super) fn interactive() -> Self {
        let (_, invalidations) = mpsc::channel();
        Self {
            entries: HashMap::new(),
            invalidations,
            watcher: None,
            loading: Loading::Background(super::loader::Loader::spawn()),
            pending: HashSet::new(),
        }
    }

    pub(super) fn poll(&mut self) -> bool {
        let Loading::Background(Some(loader)) = &self.loading else {
            return false;
        };
        let mut changed = false;
        while let Ok(reply) = loader.replies.try_recv() {
            changed = true;
            match reply {
                super::loader::Reply::Loaded(path, entries) => {
                    self.pending.remove(&path);
                    self.entries.insert(path, entries);
                }
                super::loader::Reply::Invalidated(path) => {
                    self.entries.remove(&path);
                    if let Some(parent) = path.parent() {
                        self.entries.remove(parent);
                    }
                }
            }
        }
        changed
    }

    pub(super) fn entries(&mut self, directory: &Path) -> Vec<Entry> {
        self.drain_invalidations();
        if let Some(entries) = self.entries.get(directory) {
            return entries.clone();
        }
        if let Loading::Background(Some(loader)) = &self.loading {
            if !self.pending.contains(directory)
                && loader.requests.try_send(directory.into()).is_ok()
            {
                self.pending.insert(directory.into());
            }
            return Vec::new();
        }
        if matches!(self.loading, Loading::Background(_)) {
            return Vec::new();
        }
        let entries = super::loader::read(directory);
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

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
