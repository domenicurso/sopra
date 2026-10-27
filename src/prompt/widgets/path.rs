use std::path::{Path as FsPath, PathBuf};

use super::{Widget, WidgetAction};
use crate::input::Key;

pub(super) struct Path {
    ancestors: Vec<PathBuf>,
    selected: Option<usize>,
}

impl Path {
    pub(super) fn new(cwd: PathBuf) -> Self {
        Self {
            ancestors: cwd.ancestors().map(FsPath::to_path_buf).collect(),
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

    fn render(&self) -> String {
        let Some(path) = self.ancestors.get(self.selected.unwrap_or(0)) else {
            return String::new();
        };
        let label = display_path(path);
        if self.active() {
            format!("[{label}]")
        } else {
            label
        }
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
            Key::Left | Key::Down => {
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
