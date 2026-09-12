use keel_core::{AppState, SUGGESTION_VIEWPORT_ROWS};
use keel_ui::{MAX_VISIBLE_ITEMS, PopupItem, PopupPlacement, Scene, SuggestionPopup};

use crate::abi::POPUP_MIN_ROWS;

pub(crate) fn build_scene(
    app: &AppState,
    anchor: u16,
    placement: PopupPlacement,
    completion_ms: u64,
) -> Option<Scene> {
    if !app.suggestions_visible() {
        return None;
    }
    let items = app
        .suggestions
        .iter()
        .map(|suggestion| {
            PopupItem::new(&suggestion.label, &suggestion.detail)
                .with_match_indices(suggestion.match_indices().to_vec())
        })
        .collect::<Vec<_>>();
    let footer = format!(
        "{}/{}; {}ms",
        app.selected_suggestion
            .map_or_else(|| "0".to_string(), |selected| (selected + 1).to_string()),
        items.len(),
        completion_ms,
    );
    Some(Scene::new(
        SuggestionPopup::new(footer, items, app.selected_suggestion)
            .query(app.completion_query())
            .anchor_column(anchor)
            .placement(placement)
            .viewport(
                app.suggestion_viewport_start(),
                SUGGESTION_VIEWPORT_ROWS.min(MAX_VISIBLE_ITEMS),
            ),
    ))
}

pub(crate) fn choose_popup_placement(
    requested_height: u16,
    below: u16,
    above: u16,
) -> Option<(PopupPlacement, u16)> {
    if below >= requested_height {
        return Some((PopupPlacement::Below, requested_height));
    }
    if above >= requested_height {
        return Some((PopupPlacement::Above, requested_height));
    }

    let below_height = below.min(requested_height);
    let above_height = above.min(requested_height);
    if below_height >= POPUP_MIN_ROWS || above_height >= POPUP_MIN_ROWS {
        if below_height >= above_height {
            Some((PopupPlacement::Below, below_height))
        } else {
            Some((PopupPlacement::Above, above_height))
        }
    } else {
        None
    }
}

pub(crate) fn popup_geometry(
    cursor_column: u16,
    token_width: u16,
    width: u16,
    columns: u16,
) -> (u16, u16) {
    let max_origin = columns.saturating_sub(width);
    let token_start = cursor_column.saturating_sub(token_width);
    let origin = token_start.saturating_sub(2).min(max_origin);
    let anchor = cursor_column
        .saturating_sub(origin)
        .clamp(1, width.saturating_sub(2).max(1));
    (origin, anchor)
}
