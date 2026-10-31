mod custom;
mod format;
mod settings;

mod builtins {
    use super::{Widget, WidgetAction, WidgetContext};
    include!(concat!(env!("OUT_DIR"), "/widgets.rs"));
}

use std::path::PathBuf;

use crate::input::Key;

pub(crate) enum WidgetAction {
    Ignored,
    Consumed,
    ChangeDirectory(PathBuf),
}

trait Widget {
    fn token(&self) -> &str;
    fn render(&self) -> String;
    fn markup(&self) -> bool {
        false
    }
    fn active(&self) -> bool {
        false
    }
    fn interactive(&self) -> bool {
        false
    }
    fn handle(&mut self, _key: Key) -> WidgetAction {
        WidgetAction::Ignored
    }
}

struct WidgetContext<'a> {
    cwd: &'a std::path::Path,
    options: &'a std::collections::BTreeMap<String, String>,
}

impl WidgetContext<'_> {
    fn option(&self, key: &str, default: &str) -> String {
        self.options
            .get(key)
            .map_or(default, String::as_str)
            .to_owned()
    }
}

pub(crate) struct Widgets {
    prompt_template: String,
    right_template: String,
    widgets: Vec<Box<dyn Widget>>,
}

impl Widgets {
    pub(crate) fn new(prompt: String, right: String, cwd: PathBuf) -> Self {
        let settings = match settings::Settings::read() {
            Ok(settings) => settings,
            Err(error) => {
                return Self {
                    prompt_template: format!("configuration error: {error}\n$ "),
                    right_template: String::new(),
                    widgets: Vec::new(),
                };
            }
        };
        let mut widgets = Vec::new();
        for (name, create) in builtins::REGISTRY {
            if referenced(&prompt, &right, name) {
                let options = settings.options(name);
                widgets.push(create(&WidgetContext {
                    cwd: &cwd,
                    options: &options,
                }));
            }
        }
        widgets.extend(custom::discover((&prompt, &right), &cwd, &settings));
        Self {
            prompt_template: prompt,
            right_template: right,
            widgets,
        }
    }

    pub(crate) fn render(&self) -> (String, String) {
        (
            self.render_template(&self.prompt_template),
            self.render_template(&self.right_template),
        )
    }

    fn render_template(&self, template: &str) -> String {
        format::render(template, &self.widgets)
    }

    pub(crate) fn active(&self) -> bool {
        self.widgets.iter().any(|widget| widget.active())
    }

    pub(crate) fn handle(&mut self, key: Key) -> WidgetAction {
        let active = self.widgets.iter().position(|widget| widget.active());
        if key == Key::PromptWidgets {
            return self.focus_next(active);
        }
        if let Some(index) = active {
            return self.widgets[index].handle(key);
        }
        WidgetAction::Ignored
    }

    fn focus_next(&mut self, active: Option<usize>) -> WidgetAction {
        if let Some(index) = active {
            self.widgets[index].handle(Key::Escape);
        }
        let start = active.map_or(0, |index| index + 1);
        let next = (start..self.widgets.len())
            .chain(0..start)
            .find(|index| Some(*index) != active && self.widgets[*index].interactive());
        if let Some(index) = next {
            return self.widgets[index].handle(Key::PromptWidgets);
        }
        if active.is_some() {
            WidgetAction::Consumed
        } else {
            WidgetAction::Ignored
        }
    }
}

fn referenced(left: &str, right: &str, name: &str) -> bool {
    [format!("{{{name}}}"), format!("[if {name}]")]
        .iter()
        .any(|token| left.contains(token) || right.contains(token))
}

#[cfg(test)]
mod tests;
