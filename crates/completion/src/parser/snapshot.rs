use super::discover::HelpRequest;
use crate::graph::CommandGraph;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    time::Duration,
};

const LIMIT: u64 = 2_000_000;
const FINGERPRINT: &str = include_str!(concat!(env!("OUT_DIR"), "/graph-fingerprint"));

pub(crate) struct Hit {
    pub(crate) graph: CommandGraph,
    pub(crate) stale: bool,
}

// Snapshots are derived, disposable acceleration data. Source bundles remain the inspectable evidence.
pub(crate) fn read(request: &HelpRequest) -> Option<Hit> {
    if let Some(hit) = load(path(request, false), false) {
        return Some(hit);
    }
    load(path(request, true), true)
}

fn load(path: PathBuf, provisional: bool) -> Option<Hit> {
    let metadata = fs::metadata(&path).ok()?;
    if metadata.len() > LIMIT {
        return None;
    }
    let age = metadata.modified().ok()?.elapsed().ok()?;
    if age > Duration::from_secs(7 * 24 * 3600) {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let body = bytes.strip_prefix(b"SOPRAGRF")?;
    let checksum = u64::from_le_bytes(body.get(..8)?.try_into().ok()?);
    let payload = body.get(8..)?;
    if digest(payload) != checksum {
        return None;
    }
    let graph: CommandGraph = postcard::from_bytes(payload).ok()?;
    if !graph.useful() {
        return None;
    }
    Some(Hit {
        graph,
        stale: provisional || age > super::cache::MAX_AGE,
    })
}

pub(super) fn write(request: &HelpRequest, graph: &CommandGraph) {
    let mut compact = graph.clone();
    strip_evidence(&mut compact.root);
    let Ok(payload) = postcard::to_allocvec(&compact) else {
        return;
    };
    if payload.len() as u64 + 16 > LIMIT {
        return;
    }
    let mut bytes = b"SOPRAGRF".to_vec();
    bytes.extend(digest(&payload).to_le_bytes());
    bytes.extend(payload);
    prune();
    for provisional in [false, true] {
        let path = path(request, provisional);
        let Some(parent) = path.parent() else {
            continue;
        };
        if fs::create_dir_all(parent).is_ok() {
            let _ = super::cache::replace(&path, &bytes);
        }
    }
}

fn strip_evidence(node: &mut crate::graph::CommandNode) {
    node.evidence.clear();
    for child in &mut node.subcommands {
        strip_evidence(child);
    }
}

fn path(request: &HelpRequest, provisional: bool) -> PathBuf {
    let mut hash = DefaultHasher::new();
    FINGERPRINT.hash(&mut hash);
    request.program.hash(&mut hash);
    request.args.hash(&mut hash);
    request.command.hash(&mut hash);
    if provisional {
        for name in ["PATH", "MANPATH", "HOME", "XDG_CONFIG_HOME"] {
            sopra_shell_context::var_os(name).hash(&mut hash);
        }
    } else {
        let resolved = request
            .command
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| format!("builtin:{}", request.program));
        super::identity::key(&resolved, request.command.as_deref(), &request.cwd).hash(&mut hash);
        request.cwd.hash(&mut hash);
    }
    crate::cache::command_directory("completion-graphs", &request.program)
        .join(format!("{:016x}.graph", hash.finish()))
}

fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

pub(super) fn inspect(request: &HelpRequest) -> String {
    let mut output = String::new();
    for provisional in [false, true] {
        let path = path(request, provisional);
        let kind = if provisional { "provisional" } else { "exact" };
        let state = load(path.clone(), provisional)
            .map(|hit| {
                if hit.stale {
                    "stale, usable during refresh"
                } else {
                    "fresh"
                }
            })
            .unwrap_or("unavailable or invalid");
        output.push_str(&format!(
            "Graph snapshot ({kind}): {}\nState: {state}\n",
            path.display()
        ));
    }
    output
}

fn prune() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static WRITES: AtomicUsize = AtomicUsize::new(0);
    if !WRITES.fetch_add(1, Ordering::Relaxed).is_multiple_of(32) {
        return;
    }
    let directory = std::env::temp_dir().join("sopra/completion-graphs");
    let Ok(entries) = crate::cache::inventory(&directory) else {
        return;
    };
    let mut files = entries
        .into_iter()
        .filter_map(|(path, _)| {
            if path.extension()? != "graph" {
                return None;
            }
            let modified = fs::metadata(&path).ok()?.modified().ok()?;
            Some((path, modified))
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    for (path, _) in files.into_iter().skip(510) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
