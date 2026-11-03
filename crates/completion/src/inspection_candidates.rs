use crate::{
    CompletionSource, Request,
    engine::LocalCompletion,
    inspection::{GraphMode, GraphQuery},
    parser,
};
use std::path::Path;

pub(crate) fn complete(line: &str, cursor: usize, cwd: &Path) -> Result<String, String> {
    if cursor > line.chars().count() {
        return Err("cursor exceeds the input character count".into());
    }
    let replace = crate::active_range(line, cursor);
    let request = Request {
        line: line.into(),
        cursor,
        context_line: line.into(),
        context_cursor: cursor,
        replace,
        cwd: cwd.into(),
        generation: 1,
        context_key: String::new(),
    };
    let mut local = LocalCompletion::new();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..9 {
        let result = local.complete(&request);
        let Some(help) = result.help else {
            break;
        };
        if !seen.insert(help.id()) {
            break;
        }
        let graph = parser::discover::resolve_chunk(&help);
        local.cache_graph(help.key, help.args, graph);
    }
    let mut result = local.complete(&request);
    if let Some(query) = result.provider {
        let control = parser::control::Control::new(std::time::Duration::from_millis(500));
        if let Some(items) = crate::providers::complete(&query, &control)? {
            result
                .items
                .retain(|item| item.source != CompletionSource::Filesystem);
            result.items.extend(items);
        }
    }
    render_candidates(line, cursor, &result.items)
}

fn render_candidates(
    line: &str,
    cursor: usize,
    candidates: &[crate::CompletionItem],
) -> Result<String, String> {
    let items = candidates
        .iter()
        .map(|item| {
            serde_json::json!({
                "display": item.display, "insert": item.insert, "description": item.description,
                "kind": format!("{:?}", item.kind), "source": format!("{:?}", item.source),
                "replace": { "start": item.replace.start, "end": item.replace.end },
                "suffix": item.suffix, "cursor_offset": item.cursor_offset, "group": item.group
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&serde_json::json!({"line":line,"cursor":cursor,"items":items}))
        .map_err(|error| error.to_string())
}

pub(crate) fn json(query: &GraphQuery<'_>) -> Result<String, String> {
    let request = parser::dump::request(query)?;
    let graph = if query.mode == GraphMode::Recursive {
        parser::discover::resolve(&request)
    } else {
        parser::discover::resolve_chunk(&request)
    }
    .ok_or("no completion data found")?;
    serde_json::to_string_pretty(&node(&graph.root)).map_err(|error| error.to_string())
}

fn node(node: &crate::graph::CommandNode) -> serde_json::Value {
    serde_json::json!({ "name":node.name, "aliases":node.aliases, "description":node.description,
        "exclusive":node.exclusive,
        "options":node.options.iter().map(|option| serde_json::json!({
            "names":option.names,"description":option.description,"attachment":format!("{:?}", option.attachment),
            "repeatable":option.repeatable,"value":option.value.as_ref().map(value)
        })).collect::<Vec<_>>(),
        "positionals":node.positionals.iter().map(|positional| serde_json::json!({
            "name":positional.name,"description":positional.description,"optional":positional.optional,
            "repeatable":positional.repeatable,"value":value(&positional.value)
        })).collect::<Vec<_>>(),
        "subcommands":node.subcommands.iter().map(self::node).collect::<Vec<_>>() })
}

fn value(value: &crate::graph::ValueSpec) -> serde_json::Value {
    serde_json::json!({ "name":value.name,"kind":format!("{:?}", value.kind),"choices":value.choices,
        "optional":value.optional,"literal":value.literal })
}
