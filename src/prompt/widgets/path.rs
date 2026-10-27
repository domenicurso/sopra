use std::path::{Path as FsPath, PathBuf};

use super::{Widget, WidgetAction};
use crate::input::Key;

pub(super) struct Path {
    ancestors: Vec<PathBuf>,
    selected: Option<usize>,
}

impl Path {
    pub(super) fn new(cwd: PathBuf) -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let boundary = home.filter(|home| cwd.starts_with(home));
        Self {
            ancestors: cwd
                .ancestors()
                .take_while(|path| boundary.as_ref().is_none_or(|home| path.starts_with(home)))
                .map(FsPath::to_path_buf)
                .collect(),
            selected: None,
        }
    }
}

impl Widget for Path {
    fn token(&self) -> &'static str {
        "{path}"
    }
    fn active(&self) -> bool {
        self.selected.is_some()
    }

    fn markup(&self) -> bool {
        true
    }

    fn render(&self) -> String {
        let Some(path) = self.ancestors.first() else {
            return String::new();
        };
        let label = display_path(path);
        let Some(selected) = self.selected else {
            return label.replace('[', "[[");
        };
        let prefix = display_path(&self.ancestors[selected]);
        let end = prefix.len().min(label.len());
        let start = prefix.rfind('/').map_or(0, |position| position + 1);
        let start = if start == end { 0 } else { start };
        format!(
            "[path-left]{}[/][path-selected]{}[/][path-right]{}[/]",
            label[..start].replace('[', "[["),
            label[start..end].replace('[', "[["),
            label[end..].replace('[', "[[")
        )
    }

    fn handle(&mut self, key: Key) -> WidgetAction {
        if self.selected.is_none() {
            if key == Key::PromptWidgets && !self.ancestors.is_empty() {
                self.selected = Some(0);
                return WidgetAction::Consumed;
            }
            return WidgetAction::Ignored;
        }
        match key {
            Key::Tab | Key::Right | Key::Up => {
                self.selected = Some((self.selected.unwrap_or(0) + 1) % self.ancestors.len());
            }
            Key::BackTab | Key::Left | Key::Down => {
                self.selected = Some(
                    (self.selected.unwrap_or(0) + self.ancestors.len() - 1) % self.ancestors.len(),
                );
            }
            Key::Enter => {
                let selected = self.selected.take().unwrap_or(0);
                return WidgetAction::ChangeDirectory(self.ancestors[selected].clone());
            }
            Key::Escape | Key::Cancel | Key::PromptWidgets => self.selected = None,
            _ => {}
        }
        WidgetAction::Consumed
    }
}

fn display_path(path: &FsPath) -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let text = if let Some(relative) = home
        .as_deref()
        .and_then(|home| path.strip_prefix(home).ok())
    {
        if relative.as_os_str().is_empty() {
            "~".to_string()
        } else {
            format!("~/{}", relative.display())
        }
    } else {
        path.display().to_string()
    };
    text.chars()
        .filter(|character| !character.is_control())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Path, Widget};
    use crate::{input::Key, prompt::parse};
    use ratatui::style::{Color, Modifier};
    use std::path::PathBuf;

    #[test]
    fn focus_styles_segments_without_changing_path_text() {
        let mut path = Path::new(PathBuf::from("/tmp/alpha/beta"));
        let before = path.render();
        path.handle(Key::PromptWidgets);
        let selected = parse(&path.render());
        assert_eq!(
            selected
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            before
        );
        assert_eq!(selected.last().map(|span| span.text.as_str()), Some("beta"));
        assert_eq!(
            selected.last().and_then(|span| span.style.fg),
            Some(Color::Cyan)
        );
        assert!(
            selected
                .last()
                .is_some_and(|span| span.style.add_modifier.contains(Modifier::REVERSED))
        );
        path.handle(Key::Tab);
        let parent = parse(&path.render());
        assert_eq!(
            parent
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            before
        );
        assert!(parent.iter().any(
            |span| span.text == "alpha" && span.style.add_modifier.contains(Modifier::REVERSED)
        ));
        assert!(parent.iter().any(|span| span.text == "/beta" && span.style.add_modifier.contains(Modifier::DIM)));
        path.handle(Key::BackTab);
        assert_eq!(parse(&path.render()), selected);
    }
}
