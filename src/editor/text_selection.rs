use std::{
    ops::Range,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::input::MouseKind;

#[derive(Default)]
pub(super) struct TextSelection {
    range: Option<Range<usize>>,
    anchor: Option<Range<usize>>,
    last_click: Option<(usize, Instant, u8)>,
    clicks: u8,
}

impl TextSelection {
    pub(super) fn range(&self) -> Option<Range<usize>> {
        self.range.clone().filter(|range| range.start < range.end)
    }

    pub(super) fn dragging(&self) -> bool {
        self.anchor.is_some()
    }

    pub(super) fn clear(&mut self) {
        self.range = None;
        self.anchor = None;
        self.last_click = None;
    }

    pub(super) fn mouse(
        &mut self,
        kind: MouseKind,
        offset: usize,
        buffer: &str,
        now: Instant,
    ) -> usize {
        match kind {
            MouseKind::Press => self.press(offset, buffer, now),
            MouseKind::Drag => self.drag(offset, buffer),
            MouseKind::Release => {
                self.anchor = None;
                offset
            }
        }
    }

    fn press(&mut self, offset: usize, buffer: &str, now: Instant) -> usize {
        self.clicks = self
            .last_click
            .filter(|(previous, time, _)| {
                *previous == offset
                    && now.saturating_duration_since(*time) <= Duration::from_millis(500)
            })
            .map_or(1, |(_, _, clicks)| clicks % 3 + 1);
        self.last_click = Some((offset, now, self.clicks));
        let anchor = unit_range(buffer, offset, self.clicks);
        self.range = Some(anchor.clone());
        self.anchor = Some(anchor.clone());
        anchor.end
    }

    fn drag(&mut self, offset: usize, buffer: &str) -> usize {
        let Some(anchor) = self.anchor.clone() else {
            return offset;
        };
        let target = unit_range(buffer, offset, self.clicks);
        let range = anchor.start.min(target.start)..anchor.end.max(target.end);
        self.range = Some(range.clone());
        if offset < anchor.start {
            range.start
        } else {
            range.end
        }
    }
}

impl super::EditorState {
    pub(super) fn prepare_text_selection(&mut self, key: crate::actions::Action) -> bool {
        use crate::actions::Action;
        let selected = self.text_selection.range();
        match key {
            Action::Character(_)
            | Action::Yank
            | Action::Backspace
            | Action::Delete
            | Action::WordBackspace => {
                if let Some(range) = selected {
                    self.buffer.replace_range(range.clone(), "");
                    self.cursor = range.start;
                    self.text_selection.clear();
                    self.reset_selection();
                    return matches!(
                        key,
                        Action::Backspace | Action::Delete | Action::WordBackspace
                    );
                }
                self.text_selection.clear();
            }
            Action::Left | Action::Right if selected.is_some() => {
                let range = selected.unwrap_or(0..0);
                self.cursor = if key == Action::Left {
                    range.start
                } else {
                    range.end
                };
                self.text_selection.clear();
                return true;
            }
            Action::Escape if selected.is_some() => {
                self.text_selection.clear();
                return true;
            }
            Action::Left
            | Action::Right
            | Action::WordLeft
            | Action::WordRight
            | Action::Home
            | Action::End
            | Action::KillToStart
            | Action::KillToEnd
            | Action::Tab
            | Action::BackTab => self.text_selection.clear(),
            _ => {}
        }
        false
    }
}

fn unit_range(buffer: &str, offset: usize, clicks: u8) -> Range<usize> {
    if clicks >= 3 {
        return 0..buffer.len();
    }
    if clicks == 1 || buffer.is_empty() {
        return offset..offset;
    }
    let position = if offset == buffer.len() {
        offset.saturating_sub(1)
    } else {
        offset
    };
    buffer
        .split_word_bound_indices()
        .find_map(|(start, word)| {
            (start <= position && position < start + word.len())
                .then_some(start..start + word.len())
        })
        .unwrap_or(offset..offset)
}
