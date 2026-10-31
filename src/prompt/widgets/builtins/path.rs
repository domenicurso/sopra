use sopra::{Action, Context, Event, Key, Text, Widget};
use std::path::{Path as FsPath, PathBuf};

#[derive(Default)]
struct Path {
    cwd: PathBuf,
    ancestors: Vec<PathBuf>,
    selected: Option<usize>,
}

impl Path {
    fn prepare(&mut self, context: &Context) {
        if self.cwd == context.cwd {
            return;
        }
        self.cwd = context.cwd.clone();
        let home = context.environment.get("HOME").map(PathBuf::from);
        let boundary = home.filter(|home| self.cwd.starts_with(home));
        self.ancestors = self
            .cwd
            .ancestors()
            .take_while(|path| boundary.as_ref().is_none_or(|home| path.starts_with(home)))
            .map(FsPath::to_path_buf)
            .collect();
        self.selected = None;
    }
}

impl Widget for Path {
    fn interactive(&self) -> bool {
        true
    }
    fn active(&self) -> bool {
        self.selected.is_some()
    }

    fn render(&mut self, context: &Context) -> Text {
        self.prepare(context);
        let Some(path) = self.ancestors.first() else {
            return Text::new();
        };
        let label = display_path(path, context);
        let Some(selected) = self.selected else {
            return label.into();
        };
        let prefix = display_path(&self.ancestors[selected], context);
        let end = prefix.len().min(label.len());
        let start = prefix.rfind('/').map_or(0, |position| position + 1);
        let start = if start == end { 0 } else { start };
        Text::new()
            .part(sopra::Emphasis::Normal, &label[..start])
            .part(sopra::Emphasis::Selected, &label[start..end])
            .part(sopra::Emphasis::Dim, &label[end..])
    }

    fn handle(&mut self, event: Event, context: &Context) -> Action {
        self.prepare(context);
        if event == Event::Activate && !self.ancestors.is_empty() {
            self.selected = Some(0);
            return Action::Consumed;
        }
        let Some(selected) = self.selected else {
            return Action::Ignored;
        };
        match event {
            Event::Key(Key::Tab | Key::Right | Key::Up) => {
                self.selected = Some((selected + 1) % self.ancestors.len());
            }
            Event::Key(Key::BackTab | Key::Left | Key::Down) => {
                self.selected = Some((selected + self.ancestors.len() - 1) % self.ancestors.len());
            }
            Event::Key(Key::Enter) => {
                self.selected = None;
                return Action::ChangeDirectory(self.ancestors[selected].clone());
            }
            Event::Key(Key::Escape | Key::Cancel) => self.selected = None,
            _ => {}
        }
        Action::Consumed
    }
}

fn display_path(path: &FsPath, context: &Context) -> String {
    let home = context.environment.get("HOME").map(PathBuf::from);
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

pub(crate) fn create() -> Box<dyn Widget + Send> {
    Box::new(Path::default())
}

#[cfg(test)]
#[path = "../path_tests.rs"]
mod tests;
