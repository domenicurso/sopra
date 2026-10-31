use super::EditorState;
use sopra_completion::CompletionItem;
use std::time::Duration;

impl EditorState {
    pub(crate) fn selected_index(&self) -> Option<usize> {
        self.completions.focus.selected()
    }

    pub(crate) const fn suggestion_viewport_start(&self) -> usize {
        self.completions.scroll
    }

    pub(crate) fn overlay_visible(&self) -> bool {
        self.completions.visible && !self.widgets.active()
    }

    pub(crate) fn completion_query_for(&self, item: &CompletionItem) -> String {
        if item.kind == sopra_completion::CompletionKind::History {
            return self.buffer[..self.cursor].to_string();
        }
        sopra_completion::ranking::replacement_query(&self.buffer, self.cursor, item).to_string()
    }

    pub(crate) fn completion_token_width(&self) -> u16 {
        let query = self
            .completions
            .items
            .first()
            .map_or_else(String::new, |item| self.completion_query_for(item));
        sopra_completion::ranking::completion_token_width(&self.completions.items, &query)
            .min(u16::MAX as usize) as u16
    }

    pub(crate) fn visible_items(&self) -> &[CompletionItem] {
        &self.completions.items
    }

    pub(crate) fn completion_footer(&self) -> String {
        if self.searching_history() {
            return format!(
                "History: {}/{}",
                self.selected_index().map_or(0, |i| i + 1),
                self.visible_items().len()
            );
        }
        format!(
            "{}/{}; {}",
            self.selected_index().map_or(0, |index| index + 1),
            self.completions.items.len(),
            completion_elapsed(self.completions.elapsed)
        )
    }

    pub(crate) fn completion_hint(&self) -> &'static str {
        if self.searching_history() {
            return if self.visible_items().is_empty() {
                "No matches - Esc cancel"
            } else {
                "Enter select - Esc cancel"
            };
        }
        if self.selected_index().is_some() || self.completions.items.len() == 1 {
            "Tab to accept"
        } else {
            "Tab to focus"
        }
    }
}

fn completion_elapsed(elapsed: Duration) -> String {
    if elapsed.is_zero() {
        return "pending".to_string();
    }
    let millis = elapsed.as_secs_f64() * 1_000.0;
    if millis < 0.1 {
        "<0.1ms".to_string()
    } else {
        format!("{millis:.1}ms")
    }
}
