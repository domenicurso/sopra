use super::LocalCompletion;
use crate::{Request, parser};

impl LocalCompletion {
    pub(super) fn restore(&mut self, request: &Request, invocation: &parser::Invocation) {
        if self.graphs.get(&invocation.key).is_none() {
            let help = parser::request_for(request, invocation);
            if let Some(hit) = parser::snapshot::read(&help) {
                self.graphs
                    .merge(invocation.key.clone(), Vec::new(), Some(hit.graph));
                if hit.stale {
                    self.graphs.provisional(&invocation.key);
                }
            }
        }
        let Some(entry) = self.graphs.get(&invocation.key) else {
            return;
        };
        let Some(graph) = &entry.graph else {
            return;
        };
        let (_, _, path) = parser::resolve(graph, invocation);
        if path.is_empty() || entry.hydrated.contains(&path) {
            return;
        }
        let help = parser::request_for_path(request, invocation, path.clone(), true);
        if let Some(hit) = parser::snapshot::read(&help) {
            self.graphs
                .merge(invocation.key.clone(), path, Some(hit.graph));
            if hit.stale {
                self.graphs.provisional(&invocation.key);
            }
        }
    }
}
