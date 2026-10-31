use sopra_completion::{CompletionEngine, CompletionItem};
use std::time::Duration;

#[derive(Clone, Copy, Default)]
pub(super) enum Focus {
    #[default]
    None,
    Pending,
    Selected(usize),
}
impl Focus {
    pub(super) fn selected(self) -> Option<usize> {
        if let Self::Selected(index) = self {
            Some(index)
        } else {
            None
        }
    }
    pub(super) fn armed(self) -> bool {
        !matches!(self, Self::None)
    }
    pub(super) fn select(&mut self, index: Option<usize>) {
        *self = index.map_or(
            if self.armed() {
                Self::Pending
            } else {
                Self::None
            },
            Self::Selected,
        );
    }
    pub(super) fn set_armed(&mut self, armed: bool) {
        if !armed {
            *self = Self::None;
        } else if !self.armed() {
            *self = Self::Pending;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Enabled,
    Suspended,
}

pub(super) struct State {
    pub focus: Focus,
    pub mode: Mode,
    pub visible: bool,
    pub scroll: usize,
    pub source: Vec<CompletionItem>,
    pub items: Vec<CompletionItem>,
    pub key: String,
    pub generation: u64,
    pub engine: Option<CompletionEngine>,
    pub elapsed: Duration,
    pub latency_generation: u64,
}
impl State {
    pub(super) fn new(config: &super::EditorConfig, mut engine: Option<CompletionEngine>) -> Self {
        let active = config.completion_enabled && !config.suppress_completion;
        Self {
            focus: if config.arm_completion && active {
                Focus::Selected(0)
            } else {
                Focus::None
            },
            mode: if active {
                Mode::Enabled
            } else {
                Mode::Suspended
            },
            visible: active,
            scroll: 0,
            source: Vec::new(),
            items: Vec::new(),
            key: String::new(),
            generation: engine.as_mut().map_or(0, CompletionEngine::begin_edit),
            engine,
            elapsed: Duration::ZERO,
            latency_generation: 0,
        }
    }
    pub(super) fn suspended(&self) -> bool {
        self.mode == Mode::Suspended
    }
    pub(super) fn set_suspended(&mut self, suspended: bool) {
        self.mode = if suspended {
            Mode::Suspended
        } else {
            Mode::Enabled
        };
        if suspended {
            self.focus = Focus::None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_focus_survives_missing_results_but_disarming_clears_selection() {
        let mut focus = Focus::Pending;
        focus.select(None);
        assert!(focus.armed());
        focus.select(Some(2));
        assert_eq!(focus.selected(), Some(2));
        focus.set_armed(false);
        assert_eq!(focus.selected(), None);
    }
}
