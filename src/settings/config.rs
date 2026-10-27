use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use super::schema::SettingKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Settings {
    pub(super) prompt: String,
    pub(super) rprompt: String,
    pub(super) transient_prompt: String,
    pub(super) clock_format: String,
    pub(super) cursor_animation: bool,
    pub(super) syntax_highlighting: bool,
    pub(super) completion_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            prompt: "$ ".to_string(),
            rprompt: String::new(),
            transient_prompt: "$ ".to_string(),
            clock_format: "%H:%M:%S".to_string(),
            cursor_animation: true,
            syntax_highlighting: true,
            completion_enabled: true,
        }
    }
}

impl Settings {
    pub(super) fn load(path: Option<&Path>) -> Self {
        let mut settings = Self::default();
        let Some(path) = path else {
            return settings;
        };
        let Ok(contents) = fs::read_to_string(path) else {
            return settings;
        };
        for line in contents.lines() {
            if let Some((key, value)) = line.split_once('=') {
                settings.set_serialized(key, value);
            }
        }
        settings
    }

    pub(super) fn apply_environment(&mut self) {
        for key in SettingKey::ALL {
            if let Ok(value) = env::var(key.environment_name()) {
                self.set_serialized(key.storage_name(), &value);
            }
        }
    }

    pub(super) fn value(&self, key: SettingKey) -> String {
        match key {
            SettingKey::Prompt => self.prompt.clone(),
            SettingKey::RightPrompt => self.rprompt.clone(),
            SettingKey::TransientPrompt => self.transient_prompt.clone(),
            SettingKey::ClockFormat => self.clock_format.clone(),
            SettingKey::CursorAnimation => toggle_value(self.cursor_animation),
            SettingKey::SyntaxHighlighting => toggle_value(self.syntax_highlighting),
            SettingKey::CompletionEnabled => toggle_value(self.completion_enabled),
        }
    }

    pub(super) fn set_text(&mut self, key: SettingKey, value: String) {
        match key {
            SettingKey::Prompt => self.prompt = value,
            SettingKey::RightPrompt => self.rprompt = value,
            SettingKey::TransientPrompt => self.transient_prompt = value,
            SettingKey::ClockFormat => self.clock_format = value,
            SettingKey::CursorAnimation
            | SettingKey::SyntaxHighlighting
            | SettingKey::CompletionEnabled => {}
        }
    }

    pub(super) fn toggle(&mut self, key: SettingKey) {
        match key {
            SettingKey::CursorAnimation => self.cursor_animation = !self.cursor_animation,
            SettingKey::SyntaxHighlighting => self.syntax_highlighting = !self.syntax_highlighting,
            SettingKey::CompletionEnabled => self.completion_enabled = !self.completion_enabled,
            SettingKey::Prompt
            | SettingKey::RightPrompt
            | SettingKey::TransientPrompt
            | SettingKey::ClockFormat => {}
        }
    }

    pub(super) fn save(&self, path: &Path) -> io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("Sopra config path has no parent"))?;
        fs::create_dir_all(parent)?;
        let contents = SettingKey::ALL
            .into_iter()
            .map(|key| format!("{}={}\n", key.storage_name(), self.value(key)))
            .collect::<String>();
        let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
        fs::write(&temporary, contents)?;
        fs::rename(temporary, path)
    }

    fn set_serialized(&mut self, key: &str, value: &str) {
        match key {
            "prompt" => self.prompt = value.to_string(),
            "rprompt" => self.rprompt = value.to_string(),
            "transient_prompt" => self.transient_prompt = value.to_string(),
            "clock_format" => self.clock_format = value.to_string(),
            "cursor_animation" => self.cursor_animation = parse_bool(value, self.cursor_animation),
            "syntax_highlighting" => {
                self.syntax_highlighting = parse_bool(value, self.syntax_highlighting)
            }
            "completion_enabled" => {
                self.completion_enabled = parse_bool(value, self.completion_enabled)
            }
            _ => {}
        }
    }
}

fn parse_bool(value: &str, fallback: bool) -> bool {
    match value {
        "1" | "true" | "True" | "on" | "On" | "yes" => true,
        "0" | "false" | "False" | "off" | "Off" | "no" => false,
        _ => fallback,
    }
}

fn toggle_value(value: bool) -> String {
    if value { "On" } else { "Off" }.to_string()
}

pub(super) fn path() -> Option<PathBuf> {
    let root = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(root.join("sopra/settings"))
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::{SettingKey, Settings, path};

    #[test]
    fn settings_round_trip_preserves_schema_values() {
        let file = env::temp_dir().join(format!("sopra-settings-{}", std::process::id()));
        let settings = Settings {
            prompt: "%n in %~ $ ".to_string(),
            rprompt: "right".to_string(),
            transient_prompt: "done $ ".to_string(),
            clock_format: "%H[dim]:[/]%M".to_string(),
            cursor_animation: false,
            syntax_highlighting: false,
            completion_enabled: true,
        };
        let _ = fs::remove_file(&file);
        settings.save(&file).expect("settings should save");
        let loaded = Settings::load(Some(&file));
        let _ = fs::remove_file(&file);
        assert_eq!(loaded, settings);
        assert_eq!(loaded.value(SettingKey::CompletionEnabled), "On");
    }

    #[test]
    fn missing_config_uses_struct_defaults() {
        let file = path().expect("config path").join("missing");
        let settings = Settings::load(Some(&file));
        assert!(settings.cursor_animation);
        assert!(settings.syntax_highlighting);
        assert!(settings.completion_enabled);
    }
}
