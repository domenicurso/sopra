use std::path::{Path as FsPath, PathBuf};

use super::{Widget, WidgetAction};
use crate::input::Key;

pub(super) struct Path {
    ancestors: Vec<PathBuf>,
    selected: Option<usize>,
    styles: PathStyles,
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
            styles: PathStyles::from_environment(),
        }
    }
}

struct PathStyles {
    before: String,
    selected: String,
    after: String,
}

impl PathStyles {
    fn from_environment() -> Self {
        Self {
            before: std::env::var("SOPRA_PATH_BEFORE").unwrap_or_else(|_| "{text}".into()),
            selected: std::env::var("SOPRA_PATH_SELECTED")
                .unwrap_or_else(|_| "[reverse]{text}[/]".into()),
            after: std::env::var("SOPRA_PATH_AFTER").unwrap_or_else(|_| "[dim]{text}[/]".into()),
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
        [
            self.styles
                .before
                .replace("{text}", &label[..start].replace('[', "[[")),
            self.styles
                .selected
                .replace("{text}", &label[start..end].replace('[', "[[")),
            self.styles
                .after
                .replace("{text}", &label[end..].replace('[', "[[")),
        ]
        .concat()
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
#[path = "path_tests.rs"]
mod tests;
