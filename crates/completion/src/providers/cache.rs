use super::{
    config::{Provider, Source},
    query::Query,
};
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    path::Path,
    time::Instant,
};

pub(super) struct Cache {
    entries: HashMap<u64, Entry>,
}
struct Entry {
    loaded: Instant,
    values: Vec<String>,
}

impl Cache {
    pub(super) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
    pub(super) fn get(&self, key: u64, provider: &Provider) -> Option<Vec<String>> {
        self.entries
            .get(&key)
            .filter(|entry| entry.loaded.elapsed() < provider.ttl)
            .map(|entry| entry.values.clone())
    }
    pub(super) fn put(&mut self, key: u64, values: Vec<String>) {
        if self.entries.len() >= 64
            && let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.loaded)
                .map(|(key, _)| *key)
        {
            self.entries.remove(&oldest);
        }
        self.entries.insert(
            key,
            Entry {
                loaded: Instant::now(),
                values,
            },
        );
    }
}

pub(super) fn key(provider: &Provider, query: &Query, root: &Path) -> u64 {
    let mut hash = DefaultHasher::new();
    format!("{provider:?}").hash(&mut hash);
    query.request.cwd.hash(&mut hash);
    root.hash(&mut hash);
    for file in &provider.watch {
        stamp(&root.join(file), &mut hash);
    }
    match &provider.source {
        Source::Json { file, .. } => stamp(&root.join(file), &mut hash),
        Source::Exec(argv) => {
            if let Some((resolved, executable)) =
                crate::commands::resolve(&argv[0], &query.request.cwd)
            {
                crate::parser::identity::key(&resolved, executable.as_deref(), &query.request.cwd)
                    .hash(&mut hash);
            }
        }
    }
    hash.finish()
}

fn stamp(path: &Path, hash: &mut DefaultHasher) {
    fs::metadata(path)
        .ok()
        .map(|metadata| (metadata.len(), metadata.modified().ok()))
        .hash(hash);
}
