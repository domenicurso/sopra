mod edit;

use std::path::PathBuf;

use crate::input::Key;

use super::{
    config::{self, Settings},
    schema::{SettingDefinition, SettingKey, TABS, TabDefinition},
};

pub(super) struct SettingsState {
    settings: Settings,
    tab: usize,
    selected: usize,
    editing: bool,
    cursor: usize,
    edit_original: Option<String>,
    error: Option<String>,
    config_path: Option<PathBuf>,
}

impl SettingsState {
    pub(super) fn load() -> Self {
        let config_path = config::path();
        let mut settings = Settings::load(config_path.as_deref());
        settings.apply_environment();
        Self {
            settings,
            tab: 0,
            selected: 0,
            editing: false,
            cursor: 0,
            edit_original: None,
            error: None,
            config_path,
        }
    }

    pub(super) fn current_tab(&self) -> &'static TabDefinition {
        &TABS[self.tab]
    }

    pub(super) fn tab_index(&self) -> usize {
        self.tab
    }

    pub(super) fn selected_index(&self) -> usize {
        self.selected
    }

    pub(super) fn selected_definition(&self) -> SettingDefinition {
        self.current_tab().settings[self.selected]
    }

    pub(super) fn value(&self, key: SettingKey) -> String {
        self.settings.value(key)
    }

    pub(super) fn is_editing(&self) -> bool {
        self.editing
    }

    pub(super) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(super) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(super) fn config_label(&self) -> String {
        self.config_path.as_ref().map_or_else(
            || "config unavailable".to_string(),
            |path| path.display().to_string(),
        )
    }

    pub(super) fn handle_key(&mut self, key: Key) -> bool {
        if self.editing {
            return self.handle_editing_key(key);
        }
        match key {
            Key::Up => self.selected = self.selected.saturating_sub(1),
            Key::Down => self.selected = (self.selected + 1) % self.current_tab().settings.len(),
            Key::Left => self.switch_tab(-1),
            Key::Right => self.switch_tab(1),
            Key::Tab => self.advance_setting(),
            Key::Enter => self.begin_or_toggle(),
            Key::Escape | Key::Cancel | Key::Eof => return true,
            _ => {}
        }
        false
    }

    fn switch_tab(&mut self, direction: i32) {
        self.tab = if direction < 0 {
            if self.tab == 0 {
                TABS.len() - 1
            } else {
                self.tab - 1
            }
        } else {
            (self.tab + 1) % TABS.len()
        };
        self.selected = self.selected.min(self.current_tab().settings.len() - 1);
        self.error = None;
    }

    fn advance_setting(&mut self) {
        if self.selected + 1 < self.current_tab().settings.len() {
            self.selected += 1;
        } else {
            self.switch_tab(1);
            self.selected = 0;
        }
    }

    #[cfg(test)]
    pub(super) fn test(settings: Settings) -> Self {
        Self {
            settings,
            tab: 0,
            selected: 0,
            editing: false,
            cursor: 0,
            edit_original: None,
            error: None,
            config_path: None,
        }
    }
}
