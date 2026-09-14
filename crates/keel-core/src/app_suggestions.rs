use neo_frizbee::{Config, Matcher, match_list_indices};
use unicode_width::UnicodeWidthStr;

use crate::{AppState, Suggestion, SuggestionKind};

pub const SUGGESTION_VIEWPORT_ROWS: usize = 12;

impl AppState {
    pub fn selected(&self) -> Option<&Suggestion> {
        self.selected_suggestion
            .and_then(|selected| self.suggestions.get(selected))
    }

    pub fn suggestions_visible(&self) -> bool {
        !self.buffer.text().is_empty() && !self.overlay_dismissed && !self.suggestions.is_empty()
    }

    pub fn selected_replacement(&self) -> Option<&str> {
        self.selected()
            .or_else(|| (self.suggestions.len() == 1).then(|| &self.suggestions[0]))
            .map(Suggestion::replacement)
    }

    pub fn completion_source_available(&self) -> bool {
        self.completion_source_key.is_some()
    }

    pub fn suggestion_viewport_start(&self) -> usize {
        self.suggestion_scroll
    }

    pub fn completion_query(&self) -> String {
        let cursor = self.buffer.cursor_byte_offset();
        self.buffer.text()[..cursor]
            .rsplit(char::is_whitespace)
            .next()
            .unwrap_or_default()
            .to_string()
    }

    pub fn completion_context_key(&self) -> String {
        let text = self.buffer.text();
        let cursor = self.buffer.cursor_byte_offset();
        let prefix = &text[..cursor];
        let token_start = prefix
            .char_indices()
            .rev()
            .find(|(_, character)| character.is_whitespace())
            .map_or(0, |(offset, character)| offset + character.len_utf8());
        let token_prefix = &prefix[token_start..];
        let suffix = &text[cursor..];
        let token_end = suffix.find(char::is_whitespace).unwrap_or(suffix.len());
        let line_suffix = &suffix[token_end..];
        let option_prefix = if token_prefix.starts_with('-') {
            "-"
        } else {
            ""
        };
        format!("{}{}{}", &prefix[..token_start], option_prefix, line_suffix)
    }

    pub fn completion_token_width(&self) -> u16 {
        let query = self.completion_query();
        self.completion_path_span(&query)
            .map(|span| span.width)
            .unwrap_or_else(|| query.rsplit('/').next().unwrap_or_default().width())
            .min(u16::MAX as usize) as u16
    }

    pub fn set_suggestions(&mut self, suggestions: Vec<Suggestion>) -> bool {
        self.completion_source = suggestions;
        self.completion_source_key = Some(self.current_completion_context_key.clone());
        self.ranked_query = None;
        self.refresh_suggestions()
    }

    pub fn refresh_suggestions(&mut self) -> bool {
        if self.completion_source_key.as_deref()
            != Some(self.current_completion_context_key.as_str())
        {
            let changed = !self.suggestions.is_empty();
            self.suggestions.clear();
            self.ranked_query = None;
            self.selected_suggestion = None;
            self.suggestion_scroll = 0;
            self.dirty |= changed;
            return changed;
        }
        let query = self.completion_query();
        if self.ranked_query.as_deref() == Some(query.as_str()) {
            return false;
        }
        let suggestions = rank_suggestions(&self.completion_source, &query);
        self.ranked_query = Some(query);
        let had_selection = self.selected_suggestion.is_some();
        let changed = self.suggestions != suggestions;
        self.suggestions = suggestions;
        if changed {
            self.selected_suggestion = had_selection
                .then_some(0)
                .filter(|_| !self.suggestions.is_empty());
            self.suggestion_scroll = 0;
        } else if self
            .selected_suggestion
            .is_some_and(|selected| selected >= self.suggestions.len())
        {
            self.selected_suggestion = None;
            self.suggestion_scroll = 0;
        }
        self.dirty |= changed;
        changed
    }

    pub fn move_selection(&mut self, delta: isize) -> bool {
        if !self.suggestions_visible() {
            return false;
        }

        let len = self.suggestions.len() as isize;
        let next = match self.selected_suggestion {
            Some(selected) => (selected as isize + delta).rem_euclid(len) as usize,
            None if delta < 0 => len.saturating_sub(1) as usize,
            None => 0,
        };
        self.selected_suggestion = Some(next);
        self.update_suggestion_scroll(next);
        self.dirty = true;
        true
    }

    fn update_suggestion_scroll(&mut self, selected: usize) {
        let length = self.suggestions.len();
        let max_start = length.saturating_sub(SUGGESTION_VIEWPORT_ROWS);
        if max_start == 0 {
            self.suggestion_scroll = 0;
            return;
        }

        // Keep one row available in the direction of travel. The selected item
        // moves the viewport when it reaches the last visible row, then moves
        // it back when it reaches the first visible row.
        if selected >= self.suggestion_scroll + SUGGESTION_VIEWPORT_ROWS - 1 {
            let desired_start = selected.saturating_sub(SUGGESTION_VIEWPORT_ROWS - 2);
            self.suggestion_scroll = self.suggestion_scroll.max(desired_start).min(max_start);
        } else if selected <= self.suggestion_scroll {
            self.suggestion_scroll = self.suggestion_scroll.min(selected.saturating_sub(1));
        }
    }

    pub fn dismiss_overlay(&mut self) -> bool {
        let changed = self.suggestions_visible();
        self.overlay_dismissed = true;
        self.suppressed_overlay_text = Some(self.buffer.text().to_string());
        self.dirty |= changed;
        changed
    }

    pub fn suppress_overlay_for_text(&mut self, text: impl Into<String>) {
        self.overlay_dismissed = true;
        self.suppressed_overlay_text = Some(text.into());
        self.dirty = true;
    }

    pub fn clear(&mut self) -> bool {
        let changed = !self.buffer.text().is_empty() || self.buffer.cursor() != 0;
        self.buffer = crate::EditorBuffer::new("", 0);
        self.suggestions.clear();
        self.completion_source.clear();
        self.completion_source_key = None;
        self.current_completion_context_key = self.completion_context_key();
        self.ranked_query = None;
        self.selected_suggestion = None;
        self.suggestion_scroll = 0;
        self.overlay_dismissed = false;
        self.suppressed_overlay_text = None;
        self.dirty = true;
        changed
    }

    fn completion_path_span(&self, query: &str) -> Option<CompletionPathSpan> {
        let path = &query[path_value_offset(query)..];
        if !path.contains('/') {
            return None;
        }
        let path_suggestions = self
            .completion_source
            .iter()
            .filter(|suggestion| suggestion.kind() != SuggestionKind::Generic)
            .collect::<Vec<_>>();
        if path_suggestions.is_empty() {
            return None;
        }

        let components = path_components(path);
        if components.is_empty() {
            return Some(CompletionPathSpan { width: 0 });
        }
        let resolved = resolved_path_prefix(&path_suggestions, path);
        if path.ends_with('/') && resolved == components.len() {
            return Some(CompletionPathSpan { width: 0 });
        }

        let first_unresolved = resolved.min(components.len().saturating_sub(1));
        let start = components[first_unresolved].1;
        let end = path.len() - usize::from(path.ends_with('/'));
        // Include the separator in the geometry span when the working segment
        // is empty. This keeps the popup origin on the previous unresolved
        // segment while leaving the junction under the actual cursor.
        let width = path[start..end].width() + usize::from(path.ends_with('/'));
        Some(CompletionPathSpan { width })
    }
}

fn rank_suggestions(suggestions: &[Suggestion], query: &str) -> Vec<Suggestion> {
    if query.is_empty() || suggestions.is_empty() {
        let mut ranked = suggestions.to_vec();
        compact_path_labels(&mut ranked, query);
        return ranked;
    }

    let config = Config::default();
    let generic_indices = suggestions
        .iter()
        .enumerate()
        .filter_map(|(index, suggestion)| {
            (suggestion.kind() == SuggestionKind::Generic).then_some(index)
        })
        .collect::<Vec<_>>();
    let generic_labels = generic_indices
        .iter()
        .map(|&index| suggestions[index].label.as_str())
        .collect::<Vec<_>>();
    let generic_matches = match_list_indices(query, &generic_labels, &config);
    let mut ranked = Vec::with_capacity(generic_matches.len() + suggestions.len());

    for matched in generic_matches {
        let Some(&index) = generic_indices.get(matched.index as usize) else {
            continue;
        };
        let Some(mut suggestion) = suggestions.get(index).cloned() else {
            continue;
        };
        suggestion.match_indices = matched.indices;
        ranked.push((u32::from(matched.score), index, suggestion));
    }

    for (index, suggestion) in suggestions.iter().enumerate() {
        if suggestion.kind() == SuggestionKind::Generic {
            continue;
        }
        let Some(path_match) = path_suggestion_match(suggestion, query, &config) else {
            continue;
        };
        let mut suggestion = suggestion.clone();
        suggestion.match_indices = path_match.indices;
        ranked.push((path_match.score, index, suggestion));
    }

    ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let mut ranked = ranked
        .into_iter()
        .map(|(_, _, suggestion)| suggestion)
        .collect::<Vec<_>>();
    compact_path_labels(&mut ranked, query);
    ranked
}

#[derive(Debug, Clone, Copy)]
struct CompletionPathSpan {
    width: usize,
}

struct PathMatch {
    score: u32,
    indices: Vec<usize>,
}

fn path_suggestion_match(
    suggestion: &Suggestion,
    query: &str,
    config: &Config,
) -> Option<PathMatch> {
    let query_path = &query[path_value_offset(query)..];
    let query_components = path_components(query_path);
    if query_components.is_empty() {
        return Some(PathMatch {
            score: 0,
            indices: Vec::new(),
        });
    }

    let replacement = suggestion.replacement();
    let replacement_components = path_components(&replacement[path_value_offset(replacement)..]);
    let candidate_start = candidate_component_start(
        query_path,
        query_components.len(),
        replacement_components.len(),
    )?;
    let label_path_offset = path_value_offset(&suggestion.label);
    let label_components = path_components(&suggestion.label[label_path_offset..]);
    let label_suffix_start =
        component_sequence_suffix_start(&label_components, &replacement_components);
    let mut matchers = query_components
        .iter()
        .map(|(component, _)| Matcher::new(component, config))
        .collect::<Vec<_>>();
    let mut score = 0u32;
    let mut indices = Vec::new();

    for (query_index, (_, _)) in query_components.iter().enumerate() {
        let replacement_index = candidate_start + query_index;
        let replacement_component = replacement_components.get(replacement_index)?;
        let matched = matchers[query_index].match_one_indices(replacement_component.0, 0)?;
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
        let label_prefix_bytes = suggestion.label[..label_path_offset + label_component.1].len();
        indices.extend(
            matched
                .indices
                .into_iter()
                .map(|index| label_prefix_bytes + index),
        );
    }

    Some(PathMatch { score, indices })
}

fn compact_path_labels(suggestions: &mut [Suggestion], query: &str) {
    let path_indices = suggestions
        .iter()
        .enumerate()
        .filter_map(|(index, suggestion)| {
            (suggestion.kind() != SuggestionKind::Generic).then_some(index)
        })
        .collect::<Vec<_>>();
    if path_indices.is_empty() {
        return;
    }

    let query_path = &query[path_value_offset(query)..];
    let resolved = resolved_path_prefix(
        &path_indices
            .iter()
            .filter_map(|&index| suggestions.get(index))
            .collect::<Vec<_>>(),
        query_path,
    );
    let max_removable = path_indices
        .iter()
        .filter_map(|&index| {
            let replacement = suggestions[index].replacement();
            let components = path_components(&replacement[path_value_offset(replacement)..]);
            (!components.is_empty()).then_some(components.len().saturating_sub(1))
        })
        .min()
        .unwrap_or(0);
    let removed_components = resolved.min(max_removable);
    if removed_components == 0 {
        return;
    }

    for &index in &path_indices {
        let suggestion = &mut suggestions[index];
        let label_path_offset = path_value_offset(&suggestion.label);
        let label_components = path_components(&suggestion.label[label_path_offset..]);
        let replacement = suggestion.replacement();
        let replacement_components =
            path_components(&replacement[path_value_offset(replacement)..]);
        let Some(label_suffix_start) =
            component_sequence_suffix_start(&label_components, &replacement_components)
        else {
            continue;
        };
        let Some(label_index) = removed_components.checked_sub(label_suffix_start) else {
            continue;
        };
        let Some(retained_component) = label_components.get(label_index) else {
            continue;
        };
        let label_path = &suggestion.label[label_path_offset..];
        let removed_start = label_path_offset + usize::from(label_path.starts_with('/'));
        let removed_end = label_path_offset + retained_component.1;
        if removed_end <= removed_start {
            continue;
        }
        let label = suggestion.label.clone();
        suggestion.label = format!("{}{}", &label[..removed_start], &label[removed_end..]);
        suggestion.match_indices = suggestion
            .match_indices
            .iter()
            .filter_map(|&match_index| {
                if match_index < removed_start {
                    Some(match_index)
                } else if match_index >= removed_end {
                    Some(match_index - (removed_end - removed_start))
                } else {
                    None
                }
            })
            .collect();
    }
}

fn resolved_path_prefix(suggestions: &[&Suggestion], query_path: &str) -> usize {
    let query_components = path_components(query_path);
    let mut resolved = 0;
    for (query_index, (query_component, _)) in query_components.iter().enumerate() {
        let all_resolved = suggestions.iter().all(|suggestion| {
            let replacement = suggestion.replacement();
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

fn candidate_component_start(
    query_path: &str,
    query_component_count: usize,
    candidate_component_count: usize,
) -> Option<usize> {
    if query_path.contains('/') || matches!(query_path, "." | ".." | "~") {
        Some(0)
    } else {
        candidate_component_count.checked_sub(query_component_count)
    }
}

fn component_sequence_suffix_start(
    label_components: &[(&str, usize)],
    replacement_components: &[(&str, usize)],
) -> Option<usize> {
    if label_components.len() > replacement_components.len() {
        return None;
    }
    let suffix_start = replacement_components.len() - label_components.len();
    label_components
        .iter()
        .zip(&replacement_components[suffix_start..])
        .all(|(label, replacement)| label.0 == replacement.0)
        .then_some(suffix_start)
}

fn path_value_offset(label: &str) -> usize {
    label
        .find('=')
        .filter(|_| label.starts_with('-'))
        .map_or(0, |equals| equals + 1)
}

fn path_components(path: &str) -> Vec<(&str, usize)> {
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
