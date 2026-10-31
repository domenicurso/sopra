mod background;
pub(crate) mod catalog;
mod configuration;
mod custom;
mod entry;
mod events;
pub(crate) mod inspection;
pub(crate) mod settings;

mod builtins {
    include!(concat!(env!("OUT_DIR"), "/widgets.rs"));
}

use crate::actions::Action as Key;
use crate::prompt::{PromptSpan, template::Template};
use entry::Entry;
use sopra::Action;
use sopra::{Event, Key as WidgetKey};
use std::collections::BTreeMap;
#[cfg(test)]
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Idle,
    Active(usize),
    Awaiting(usize),
}

pub(crate) struct Widgets {
    prompt_template: Template,
    right_template: Template,
    widgets: Vec<Entry>,
    focused: Focus,
}

impl Widgets {
    #[cfg(test)]
    pub(crate) fn new(prompt: String, right: String, cwd: PathBuf) -> Self {
        let settings = settings::Settings::default();
        // These fixtures use valid templates and built-in widget names.
        Self::configured(prompt, right, cwd, &settings).expect("valid widget fixture")
    }

    pub(crate) fn render(&mut self) -> (Vec<PromptSpan>, Vec<PromptSpan>) {
        let rendered = self
            .widgets
            .iter_mut()
            .map(|entry| (entry.name.clone(), entry.render()))
            .collect::<BTreeMap<_, _>>();
        (
            self.prompt_template.render(&rendered),
            self.right_template.render(&rendered),
        )
    }

    pub(crate) fn update(&mut self, cwd: &std::path::Path, settings: &settings::Settings) {
        for entry in &mut self.widgets {
            entry.update(cwd, settings.definition(&entry.name));
        }
    }

    pub(crate) fn active(&self) -> bool {
        self.focused != Focus::Idle
    }

    pub(crate) fn pending(&self) -> bool {
        matches!(self.focused, Focus::Awaiting(_))
    }

    pub(crate) fn poll_action(&mut self) -> Option<Action> {
        for (index, entry) in self.widgets.iter().enumerate() {
            while let Some((event, action)) = entry.widget.poll_action() {
                if event == Event::Key(WidgetKey::Enter) && self.focused == Focus::Awaiting(index) {
                    self.focused = if entry.widget.active() {
                        Focus::Active(index)
                    } else {
                        Focus::Idle
                    };
                } else if self.focused == Focus::Active(index) && !entry.widget.active() {
                    self.focused = Focus::Idle;
                }
                if matches!(action, Action::ChangeDirectory(_)) {
                    self.focused = Focus::Idle;
                    return Some(action);
                }
            }
        }
        None
    }

    pub(crate) fn handle(&mut self, key: Key) -> Action {
        if matches!(self.focused, Focus::Awaiting(_)) {
            return Action::Consumed;
        }
        let active = if let Focus::Active(index) = self.focused {
            Some(index)
        } else {
            None
        };
        if key == Key::PromptWidgets {
            return self.focus_next(active);
        }
        let Some(index) = active else {
            return Action::Ignored;
        };
        self.focused = match key {
            Key::Enter => Focus::Awaiting(index),
            Key::Escape | Key::Cancel => Focus::Idle,
            _ => self.focused,
        };
        events::event(key).map_or(Action::Ignored, |event| self.widgets[index].handle(event))
    }

    fn focus_next(&mut self, active: Option<usize>) -> Action {
        if let Some(index) = active {
            self.widgets[index].handle(Event::Key(WidgetKey::Escape));
        }
        let start = active.map_or(0, |index| index + 1);
        let focused = (start..self.widgets.len())
            .chain(0..start)
            .find(|index| Some(*index) != active && self.widgets[*index].widget.interactive());
        self.focused = focused.map_or(Focus::Idle, Focus::Active);
        if let Some(index) = focused {
            return self.widgets[index].handle(Event::Activate);
        }
        if active.is_some() {
            Action::Consumed
        } else {
            Action::Ignored
        }
    }
}

fn referenced(left: &Template, right: &Template, name: &str) -> bool {
    left.references()
        .chain(right.references())
        .any(|reference| reference == name)
}

#[cfg(test)]
mod tests;
