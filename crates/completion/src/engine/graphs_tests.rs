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
#[test]
fn stale_positive_graphs_remain_usable_and_failed_refreshes_keep_the_last_good_graph() {
    let mut graphs = Graphs::new();
    graphs.merge(
        "tool".into(),
        Vec::new(),
        Some(CommandGraph {
            root: crate::graph::CommandNode::named("tool"),
        }),
    );
    graphs
        .entries
        .get_mut("tool")
        .expect("inserted graph")
        .loaded = Instant::now() - MAX_AGE - std::time::Duration::from_secs(1);
    assert!(
        graphs
            .get("tool")
            .expect("stale graph remains available")
            .stale()
    );
    graphs.merge("tool".into(), Vec::new(), None);
    assert!(
        graphs
            .get("tool")
            .expect("failed refresh retained graph")
            .graph
            .is_some()
    );
}
