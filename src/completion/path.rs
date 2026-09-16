mod labels;

pub(super) use labels::compact_labels;

pub(super) fn resolved_path_prefix(items: &[&CompletionItem], query_path: &str) -> usize {
    labels::resolved_path_prefix(items, query_path)
}

use neo_frizbee::{Config, Matcher};

use super::CompletionItem;

pub(super) fn match_item(
    item: &CompletionItem,
    query: &str,
    config: &Config,
) -> Option<(u32, Vec<usize>)> {
    let query_path = &query[path_value_offset(query)..];
    let query_components = path_components(query_path);
    if query_components.is_empty() {
        return Some((0, Vec::new()));
    }
    let replacement = &item.insert;
    let replacement_path = &replacement[path_value_offset(replacement)..];
    let replacement_components = path_components(replacement_path);
    let candidate_start = candidate_component_start(
        query_path,
        query_components.len(),
        replacement_components.len(),
    )?;
    let label_offset = path_value_offset(&item.display);
    let label_components = path_components(&item.display[label_offset..]);
    let label_suffix_start =
        component_sequence_suffix_start(&label_components, &replacement_components);
    let mut score = 0_u32;
    let mut indices = Vec::new();

    for (query_index, (query_component, _)) in query_components.iter().enumerate() {
        let replacement_index = candidate_start + query_index;
        let candidate = replacement_components.get(replacement_index)?;
        let matched = Matcher::new(query_component, config).match_one_indices(candidate.0, 0)?;
        score = score.saturating_add(u32::from(matched.score));
        let Some(suffix_start) = label_suffix_start else {
            continue;
        };
        let Some(label_index) = replacement_index.checked_sub(suffix_start) else {
            continue;
        };
        let Some(label_component) = label_components.get(label_index) else {
            continue;
        };
        let label_start = label_offset + label_component.1;
        indices.extend(matched.indices.into_iter().map(|index| label_start + index));
    }
    Some((score, indices))
}

fn candidate_component_start(
    query_path: &str,
    query_count: usize,
    candidate_count: usize,
) -> Option<usize> {
    if query_path.contains('/') || matches!(query_path, "." | ".." | "~") {
        Some(0)
    } else {
        candidate_count.checked_sub(query_count)
    }
}

fn component_sequence_suffix_start(
    label: &[(&str, usize)],
    replacement: &[(&str, usize)],
) -> Option<usize> {
    if label.len() > replacement.len() {
        return None;
    }
    let start = replacement.len() - label.len();
    label
        .iter()
        .zip(&replacement[start..])
        .all(|(left, right)| left.0 == right.0)
        .then_some(start)
}

pub(super) fn path_value_offset(value: &str) -> usize {
    value
        .find('=')
        .filter(|_| value.starts_with('-'))
        .map_or(0, |equals| equals + 1)
}

pub(super) fn path_components(path: &str) -> Vec<(&str, usize)> {
    let mut components = Vec::new();
    let mut start = 0;
    for (separator, _) in path.match_indices('/') {
        if separator > start {
            components.push((&path[start..separator], start));
        }
        start = separator + 1;
    }
    if start < path.len() {
        components.push((&path[start..], start));
    }
    components
}

#[cfg(test)]
mod tests {
    use neo_frizbee::Config;

    use super::{CompletionItem, match_item};
    use crate::completion::CompletionKind;

    #[test]
    fn path_matching_scores_each_unresolved_segment() {
        let item = CompletionItem::new(
            "./crates/keel-core/src/",
            "",
            "./crates/keel-core/src/",
            CompletionKind::Directory,
        );
        let (score, indices) = match_item(&item, "./cr/kc", &Config::default()).expect("match");
        assert!(score > 0);
        assert!(indices.contains(&0));
        assert!(indices.contains(&2));
        assert!(indices.contains(&9));
    }
}
