mod clock;
mod git;
mod path;

use std::path::PathBuf;

use crate::input::Key;

pub(crate) enum WidgetAction {
    Ignored,
    Consumed,
    ChangeDirectory(PathBuf),
}

trait Widget {
    fn token(&self) -> &'static str;
    fn render(&self) -> String;
    fn active(&self) -> bool {
        false
    }
    fn handle(&mut self, _key: Key) -> WidgetAction {
        WidgetAction::Ignored
    }
}

pub(crate) struct Widgets {
    prompt_template: String,
    right_template: String,
    widgets: Vec<Box<dyn Widget>>,
}

impl Widgets {
    pub(crate) fn new(prompt: String, right: String, cwd: PathBuf) -> Self {
        let git_enabled = prompt.contains("{git}") || right.contains("{git}");
        let git = git::Git::new(&cwd, git_enabled);
        Self {
            prompt_template: prompt,
            right_template: right,
            widgets: vec![
                Box::new(clock::Clock),
                Box::new(git),
                Box::new(path::Path::new(cwd)),
            ],
        }
    }

    pub(crate) fn render(&self) -> (String, String) {
        (
            self.render_template(&self.prompt_template),
            self.render_template(&self.right_template),
        )
    }

    fn render_template(&self, template: &str) -> String {
        let mut output = String::new();
        let mut remaining = template;
        while !remaining.is_empty() {
            if let Some(widget) = self
                .widgets
                .iter()
                .find(|widget| remaining.starts_with(widget.token()))
            {
                output.push_str(&widget.render());
                remaining = &remaining[widget.token().len()..];
            } else if let Some(character) = remaining.chars().next() {
                output.push(character);
                remaining = &remaining[character.len_utf8()..];
            }
        }
        output
    }

    pub(crate) fn active(&self) -> bool {
        self.widgets.iter().any(|widget| widget.active())
    }

    pub(crate) fn handle(&mut self, key: Key) -> WidgetAction {
        for widget in &mut self.widgets {
            let present = self.prompt_template.contains(widget.token())
                || self.right_template.contains(widget.token());
            if present || widget.active() {
                let action = widget.handle(key);
                if !matches!(action, WidgetAction::Ignored) {
                    return action;
                }
            }
        }
        WidgetAction::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::{WidgetAction, Widgets};
    use crate::input::Key;
    use std::path::PathBuf;

    #[test]
    fn path_widget_selects_parents_and_can_cancel() {
        let mut widgets =
            Widgets::new("{path} $ ".into(), String::new(), PathBuf::from("/one/two"));
        assert!(matches!(
            widgets.handle(Key::PromptWidgets),
            WidgetAction::Consumed
        ));
        assert!(widgets.render().0.starts_with("[/one/two]"));
        widgets.handle(Key::Tab);
        assert!(widgets.render().0.starts_with("[/one]"));
        assert!(
            matches!(widgets.handle(Key::Enter), WidgetAction::ChangeDirectory(path) if path == std::path::Path::new("/one"))
        );
        widgets.handle(Key::PromptWidgets);
        widgets.handle(Key::Escape);
        assert!(matches!(widgets.handle(Key::Enter), WidgetAction::Ignored));
    }

    #[test]
    fn only_registered_tokens_in_the_template_are_expanded() {
        let mut widgets = Widgets::new(
            "{path} {unknown}".into(),
            String::new(),
            PathBuf::from("/one/{clock}"),
        );
        assert_eq!(widgets.render().0, "/one/{clock} {unknown}");
        widgets = Widgets::new("plain".into(), String::new(), PathBuf::from("/one"));
        assert!(matches!(
            widgets.handle(Key::PromptWidgets),
            WidgetAction::Ignored
        ));
    }
}
