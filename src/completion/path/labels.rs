use super::super::{CompletionItem, CompletionKind};
use super::{
    candidate_component_start, component_sequence_suffix_start, path_components, path_value_offset,
};

pub(crate) fn compact_labels(items: &mut [CompletionItem], query: &str) {
    let path_indices = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (item.kind != CompletionKind::Generic).then_some(index))
        .collect::<Vec<_>>();
    if path_indices.is_empty() {
        return;
    }
    let query_path = &query[path_value_offset(query)..];
    let references = path_indices
        .iter()
        .filter_map(|&index| items.get(index))
        .collect::<Vec<_>>();
    let removed = resolved_path_prefix(&references, query_path).min(
        path_indices
            .iter()
            .filter_map(|&index| {
                let replacement = &items[index].replacement;
                let components = path_components(&replacement[path_value_offset(replacement)..]);
                (!components.is_empty()).then_some(components.len().saturating_sub(1))
            })
            .min()
            .unwrap_or(0),
    );
    if removed == 0 {
        return;
    }
    for &index in &path_indices {
        compact_item(&mut items[index], removed);
    }
}

fn compact_item(item: &mut CompletionItem, removed: usize) {
    let label_offset = path_value_offset(&item.label);
    let label_components = path_components(&item.label[label_offset..]);
    let replacement = &item.replacement;
    let replacement_components = path_components(&replacement[path_value_offset(replacement)..]);
    let Some(suffix_start) =
        component_sequence_suffix_start(&label_components, &replacement_components)
    else {
        return;
    };
    let Some(label_index) = removed.checked_sub(suffix_start) else {
        return;
    };
    let Some(retained) = label_components.get(label_index) else {
        return;
    };
    let path_start = label_offset + usize::from(item.label[label_offset..].starts_with('/'));
    let path_end = label_offset + retained.1;
    if path_end <= path_start {
        return;
    }
    let old_label = item.label.clone();
    item.label = format!("{}{}", &old_label[..path_start], &old_label[path_end..]);
    item.match_indices = item
        .match_indices
        .iter()
        .filter_map(|&index| {
            if index < path_start {
                Some(index)
            } else if index >= path_end {
                Some(index - (path_end - path_start))
            } else {
                None
            }
        })
        .collect();
}

fn resolved_path_prefix(items: &[&CompletionItem], query_path: &str) -> usize {
    let query_components = path_components(query_path);
    let mut resolved = 0;
    for (query_index, (query_component, _)) in query_components.iter().enumerate() {
        let all_resolved = items.iter().all(|item| {
            let replacement = &item.replacement;
            let components = path_components(&replacement[path_value_offset(replacement)..]);
            let Some(start) =
                candidate_component_start(query_path, query_components.len(), components.len())
            else {
                return false;
            };
            components
                .get(start + query_index)
                .is_some_and(|component| component.0 == *query_component)
        });
        if !all_resolved {
            break;
        }
        resolved += 1;
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::super::super::{CompletionItem, CompletionKind};
    use super::compact_labels;

    #[test]
    fn compacting_keeps_replacements_and_shifts_highlights() {
        let mut items = vec![
            CompletionItem::new(
                "./crates/keel-core",
                "",
                "./crates/keel-core",
                CompletionKind::Directory,
            ),
            CompletionItem::new(
                "./crates/keel-ui",
                "",
                "./crates/keel-ui",
                CompletionKind::Directory,
            ),
        ];
        items[0].match_indices = vec![2, 10];
        items[1].match_indices = vec![2, 10];
        compact_labels(&mut items, "./crates/k");
        assert_eq!(items[0].label, "keel-core");
        assert_eq!(items[0].replacement, "./crates/keel-core");
        assert_eq!(items[0].match_indices, vec![1]);
    }
}
