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
            .is_some_and(|entry| entry.loaded.elapsed() > MAX_AGE)
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
        if path.is_empty() {
            self.make_room(&key);
            self.entries.insert(
                key.clone(),
                Entry {
                    graph,
                    hydrated: HashSet::new(),
                    loaded: Instant::now(),
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
        self.entries
            .retain(|_, entry| entry.loaded.elapsed() <= MAX_AGE);
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

impl Entry {
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
mod tests {
    use super::*;
    use crate::graph::CommandNode;

    #[test]
    fn expiration_discards_negative_results_and_descendant_hydration_together() {
        let mut graphs = Graphs::new();
        graphs.merge("tool".into(), Vec::new(), None);
        graphs.merge("tool".into(), vec!["old".into()], None);
        let entry = graphs.entries.get_mut("tool").expect("inserted root");
        entry.loaded = Instant::now() - MAX_AGE - std::time::Duration::from_secs(1);
        assert!(graphs.get("tool").is_none());
        graphs.merge("tool".into(), Vec::new(), None);
        assert!(
            !graphs
                .get("tool")
                .expect("new root")
                .hydrated
                .contains(&vec!["old".into()])
        );
    }

    #[test]
    fn eviction_keeps_the_most_recent_graphs_and_bounds_negative_entries() {
        let mut graphs = Graphs::new();
        for index in 0..=CAPACITY {
            graphs.merge(index.to_string(), Vec::new(), None);
        }
        assert_eq!(graphs.entries.len(), CAPACITY);
        assert!(graphs.get("0").is_none());
        assert!(graphs.get(&CAPACITY.to_string()).is_some());
    }

    #[test]
    fn nested_fragments_extend_the_existing_root_and_do_not_mark_siblings_loaded() {
        let root = CommandGraph {
            root: CommandNode {
                subcommands: vec![CommandNode::named("dev"), CommandNode::named("debug")],
                ..CommandNode::named("tool")
            },
        };
        let nested = CommandGraph {
            root: CommandNode {
                subcommands: vec![CommandNode::named("build")],
                ..CommandNode::named("tool")
            },
        };
        let mut graphs = Graphs::new();
        graphs.merge("tool".into(), Vec::new(), Some(root));
        let children = graphs.merge("tool".into(), vec!["dev".into()], Some(nested));
        assert_eq!(children, [(vec!["dev".into(), "build".into()], true)]);
        let entry = graphs.get("tool").expect("root graph was retained");
        assert!(!entry.hydrated.contains(&vec!["debug".into()]));
        assert!(
            entry
                .graph
                .as_ref()
                .and_then(|root| root.root.find(&["dev".into(), "build".into()]))
                .is_some()
        );
    }
}
