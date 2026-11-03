use crate::{graph::CommandGraph, parser::cache::MAX_AGE};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

const CAPACITY: usize = 128;

pub(super) struct Graphs {
    entries: HashMap<String, Entry>,
}

pub(super) struct Entry {
    pub(super) graph: Option<CommandGraph>,
    pub(super) hydrated: HashSet<Vec<String>>,
    loaded: Instant,
    provisional: bool,
}

impl Graphs {
    pub(super) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(super) fn get(&mut self, key: &str) -> Option<&Entry> {
        if self
            .entries
            .get(key)
            .is_some_and(|entry| entry.graph.is_none() && entry.loaded.elapsed() > MAX_AGE)
        {
            self.entries.remove(key);
        }
        self.entries.get(key)
    }

    pub(super) fn merge(
        &mut self,
        key: String,
        path: Vec<String>,
        graph: Option<CommandGraph>,
    ) -> Vec<(Vec<String>, bool)> {
        if path.is_empty()
            && graph.is_none()
            && self
                .entries
                .get(&key)
                .is_some_and(|entry| entry.graph.is_some())
        {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.loaded = Instant::now();
                entry.provisional = false;
            }
            return Vec::new();
        }
        if path.is_empty() {
            self.make_room(&key);
            self.entries.insert(
                key.clone(),
                Entry {
                    graph,
                    hydrated: HashSet::new(),
                    loaded: Instant::now(),
                    provisional: false,
                },
            );
        } else if let Some(entry) = self.entries.get_mut(&key)
            && let (Some(root), Some(fragment)) = (&mut entry.graph, graph)
        {
            root.merge_at(&path, fragment.root);
        }
        let Some(entry) = self.entries.get_mut(&key) else {
            return Vec::new();
        };
        entry.hydrated.insert(path.clone());
        entry.children(&path)
    }

    fn make_room(&mut self, key: &str) {
        if self.entries.len() < CAPACITY || self.entries.contains_key(key) {
            return;
        }
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.loaded)
            .map(|(key, _)| key.clone());
        if let Some(key) = oldest {
            self.entries.remove(&key);
        }
    }
}

impl Graphs {
    pub(super) fn provisional(&mut self, key: &str) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.provisional = true;
        }
    }
}

impl Entry {
    pub(super) fn stale(&self) -> bool {
        self.provisional || self.loaded.elapsed() > MAX_AGE
    }

    fn children(&self, path: &[String]) -> Vec<(Vec<String>, bool)> {
        let Some(node) = self.graph.as_ref().and_then(|graph| graph.root.find(path)) else {
            return Vec::new();
        };
        node.subcommands
            .iter()
            .filter_map(|child| {
                let mut child_path = path.to_vec();
                child_path.push(child.name.clone());
                let discover = child.subcommands.is_empty() && child.positionals.is_empty();
                (!self.hydrated.contains(&child_path)).then_some((child_path, discover))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "graphs_tests.rs"]
mod tests;
