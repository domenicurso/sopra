mod ansi;
mod frames;

use keel_ui::{PopupItem, Scene, SuggestionPopup};

fn popup(items: &[&str]) -> Scene {
    Scene::new(
        SuggestionPopup::new(
            format!("1/{}; Tab to accept", items.len()),
            items.iter().map(|item| PopupItem::new(*item, "")).collect(),
            Some(0),
        )
        .query("git"),
    )
}
