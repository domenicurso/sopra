use crate::completion::{CompletionItem, CompletionKind, CompletionSource};

pub(super) fn items(local: &[CompletionItem], zshrs: &[CompletionItem]) -> Vec<CompletionItem> {
    let local = local
        .iter()
        .filter(|item| item.source != CompletionSource::Zshrs)
        .cloned()
        .collect::<Vec<_>>();
    let rust_filesystem = local.iter().any(|item| {
        item.source == CompletionSource::Filesystem
            && matches!(item.kind, CompletionKind::File | CompletionKind::Directory)
    });
    let mut merged = local;
    for item in zshrs {
        if rust_filesystem && matches!(item.kind, CompletionKind::File | CompletionKind::Directory)
        {
            continue;
        }
        if let Some(existing) = merged.iter_mut().find(|existing| same_item(existing, item)) {
            if item.description.is_some() || item.location.is_some() {
                *existing = item.clone();
            }
        } else {
            merged.push(item.clone());
        }
    }
    merged
}

pub(super) fn response_metadata(
    previous: &[CompletionItem],
    response: Vec<CompletionItem>,
) -> Vec<CompletionItem> {
    response
        .into_iter()
        .map(|mut item| {
            if let Some(previous) = previous
                .iter()
                .find(|candidate| same_item(candidate, &item))
            {
                if item.description.is_none() {
                    item.description = previous.description.clone();
                }
                if item.location.is_none() {
                    item.location = previous.location.clone();
                }
                if item.group.is_none() {
                    item.group = previous.group.clone();
                }
            }
            item
        })
        .collect()
}

fn same_item(left: &CompletionItem, right: &CompletionItem) -> bool {
    left.display == right.display && left.insert == right.insert && left.kind == right.kind
}

#[cfg(test)]
mod tests {
    use super::{items, response_metadata};
    use crate::completion::{CompletionItem, CompletionKind};

    #[test]
    fn zshrs_metadata_replaces_a_local_placeholder() {
        let local = [CompletionItem::new(
            "git",
            "path",
            "git",
            CompletionKind::Generic,
        )];
        let zshrs = [CompletionItem::new(
            "git",
            "/usr/bin/git",
            "git",
            CompletionKind::Generic,
        )];
        let merged = items(&local, &zshrs);
        assert_eq!(merged[0].description.as_deref(), Some("/usr/bin/git"));
    }

    #[test]
    fn partial_zshrs_updates_keep_metadata_from_an_earlier_response() {
        let previous = vec![CompletionItem::new(
            "git",
            "/usr/bin/git",
            "git",
            CompletionKind::Generic,
        )];
        let response = vec![CompletionItem::new(
            "git",
            "",
            "git",
            CompletionKind::Generic,
        )];
        let merged = response_metadata(&previous, response);
        assert_eq!(merged[0].description.as_deref(), Some("/usr/bin/git"));
    }
}
