use std::{io, ops::Range};

use crate::input::Key;

use super::super::schema::{SettingKey, SettingKind};
use super::SettingsState;

impl SettingsState {
    pub(super) fn handle_editing_key(&mut self, key: Key) -> bool {
        match key {
            Key::Character(character) if !character.is_control() => self.insert(character),
            Key::Backspace => self.delete_previous(),
            Key::Delete => self.delete_next(),
            Key::Left => self.cursor = previous_boundary(&self.edit_value(), self.cursor),
            Key::Right => self.cursor = next_boundary(&self.edit_value(), self.cursor),
            Key::Home => self.cursor = 0,
            Key::End => self.cursor = self.edit_value().len(),
            Key::Enter => self.commit(),
            Key::Escape => self.cancel_edit(),
            Key::Cancel | Key::Eof => return true,
            _ => {}
        }
        false
    }

    pub(super) fn begin_or_toggle(&mut self) {
        let definition = self.selected_definition();
        match definition.kind {
            SettingKind::Text => {
                self.editing = true;
                self.edit_original = Some(self.settings.value(definition.key));
                self.cursor = self.edit_value().len();
                self.error = None;
            }
            SettingKind::Toggle => self.toggle(definition.key),
        }
    }

    fn insert(&mut self, character: char) {
        let mut value = self.edit_value();
        value.insert(self.cursor, character);
        self.cursor += character.len_utf8();
        self.settings
            .set_text(self.selected_definition().key, value);
    }

    fn delete_previous(&mut self) {
        let value = self.edit_value();
        let start = previous_boundary(&value, self.cursor);
        self.replace_edit_value(value, start..self.cursor, start);
    }

    fn delete_next(&mut self) {
        let value = self.edit_value();
        let end = next_boundary(&value, self.cursor);
        self.replace_edit_value(value, self.cursor..end, self.cursor);
    }

    fn replace_edit_value(&mut self, mut value: String, range: Range<usize>, cursor: usize) {
        value.replace_range(range, "");
        self.settings
            .set_text(self.selected_definition().key, value);
        self.cursor = cursor;
    }

    fn commit(&mut self) {
        match self.save() {
            Ok(()) => {
                self.editing = false;
                self.edit_original = None;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn cancel_edit(&mut self) {
        if let Some(value) = self.edit_original.take() {
            self.settings
                .set_text(self.selected_definition().key, value);
        }
        self.editing = false;
        self.error = None;
    }

    fn toggle(&mut self, key: SettingKey) {
        self.settings.toggle(key);
        if let Err(error) = self.save() {
            self.settings.toggle(key);
            self.error = Some(error.to_string());
        } else {
            self.error = None;
        }
    }

    fn save(&self) -> io::Result<()> {
        let Some(path) = &self.config_path else {
            return Err(io::Error::other(
                "Sopra could not determine a config directory",
            ));
        };
        self.settings.save(path)
    }

    fn edit_value(&self) -> String {
        self.settings.value(self.selected_definition().key)
    }
}

fn previous_boundary(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn next_boundary(value: &str, cursor: usize) -> usize {
    value[cursor..]
        .chars()
        .next()
        .map_or(cursor, |character| cursor + character.len_utf8())
}
