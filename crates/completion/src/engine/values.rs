use crate::{
    CompletionItem, CompletionKind, CompletionSource, Request,
    filesystem::FilesystemEngine,
    graph::{ValueKind, ValueSpec},
};
use neo_frizbee::{Config, Matcher};
use std::ops::Range;

pub(super) fn items(
    value: &ValueSpec,
    input: (&str, Range<usize>),
    filesystem: &mut FilesystemEngine,
    request: &Request,
) -> Vec<CompletionItem> {
    let (query, replace) = input;
    if !value.choices.is_empty() {
        if let Some(items) = assignment_items(&value.choices, query, replace.clone()) {
            return items;
        }
        return value
            .choices
            .iter()
            .filter(|choice| matches_query(choice, query))
            .map(|choice| {
                CompletionItem::with_range(
                    choice,
                    choice,
                    None,
                    CompletionKind::Value,
                    replace.clone(),
                    CompletionSource::CommandIndex,
                )
            })
            .collect();
    }
    match value.kind {
        ValueKind::File | ValueKind::Directory => filesystem.complete(request, query),
        _ => Vec::new(),
    }
}

fn assignment_items(
    choices: &[String],
    query: &str,
    replace: Range<usize>,
) -> Option<Vec<CompletionItem>> {
    let assignments = choices
        .iter()
        .map(|choice| choice.split_once('='))
        .collect::<Option<Vec<_>>>()?;
    if assignments.iter().any(|(key, _)| key.is_empty()) {
        return None;
    }

    if let Some((key, value_query)) = query.split_once('=') {
        let prefix = format!("{key}=");
        let value_replace = replace.start + prefix.len()..replace.end;
        return Some(
            assignments
                .iter()
                .filter(|(candidate_key, value)| {
                    *candidate_key == key && matches_query(value, value_query)
                })
                .map(|(_, value)| {
                    CompletionItem::with_range(
                        *value,
                        *value,
                        None,
                        CompletionKind::Value,
                        value_replace.clone(),
                        CompletionSource::CommandIndex,
                    )
                })
                .collect(),
        );
    }

    Some(assignment_keys(&assignments, query, replace))
}

fn assignment_keys(
    assignments: &[(&str, &str)],
    query: &str,
    replace: Range<usize>,
) -> Vec<CompletionItem> {
    let mut keys = assignments
        .iter()
        .map(|(key, _)| format!("{key}="))
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter()
        .filter(|key| matches_query(key, query))
        .map(|key| {
            CompletionItem::with_range(
                &key,
                &key,
                None,
                CompletionKind::Value,
                replace.clone(),
                CompletionSource::CommandIndex,
            )
        })
        .collect()
}

pub(super) fn matches_query(candidate: &str, query: &str) -> bool {
    query.is_empty()
        || Matcher::new(query, &Config::default())
            .match_one_indices(candidate, 0)
            .is_some()
}
