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
        if let Some(parent) = path.parent()
            && let Ok(contents) = fs::read_to_string(parent.join("settings"))
        {
            for line in contents.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    settings.set_serialized(key, value);
                }
            }
        }
        if let Ok(config) = crate::prompt::config::PromptConfig::read(path) {
            if let Some(value) = config.left {
                settings.prompt = value;
            }
            if let Some(value) = config.right {
                settings.rprompt = value;
            }
            if let Some(value) = config.transient {
                settings.transient_prompt = value;
            }
            if let Some(value) = config.clock_format {
                settings.clock_format = value;
            }
            if let Some(value) = config.cursor_animation {
                settings.cursor_animation = value;
            }
            if let Some(value) = config.syntax_highlighting {
                settings.syntax_highlighting = value;
            }
            if let Some(value) = config.completion_enabled {
                settings.completion_enabled = value;
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

    pub(super) fn save(&self, path: &Path, changed: SettingKey) -> io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("Sopra config path has no parent"))?;
        fs::create_dir_all(parent)?;
        super::migration::migrate(path)?;
        let mut config = crate::prompt::config::PromptConfig::default();
        match changed {
            SettingKey::Prompt => config.left = Some(self.prompt.clone()),
            SettingKey::RightPrompt => config.right = Some(self.rprompt.clone()),
            SettingKey::TransientPrompt => config.transient = Some(self.transient_prompt.clone()),
            SettingKey::ClockFormat => config.clock_format = Some(self.clock_format.clone()),
            SettingKey::CursorAnimation => config.cursor_animation = Some(self.cursor_animation),
            SettingKey::SyntaxHighlighting => {
                config.syntax_highlighting = Some(self.syntax_highlighting)
            }
            SettingKey::CompletionEnabled => {
                config.completion_enabled = Some(self.completion_enabled)
            }
        }
        crate::prompt::config::update(path, &config)
    }

    fn set_serialized(&mut self, key: &str, value: &str) {
        match key {
            "prompt" => self.prompt = value.to_string(),
            "prompt_hex" => {
                if let Some(prompt) = decode_prompt(value) {
                    self.prompt = prompt;
                }
            }
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

pub(super) fn decode_prompt(value: &str) -> Option<String> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digits = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(digits, 16).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
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
    Some(root.join("sopra/config.toml"))
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
