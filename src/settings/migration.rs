use super::config::decode_prompt;
use crate::prompt::config::{self, PromptConfig};
use std::{fs, io, path::Path};

pub(super) fn migrate(path: &Path) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let prompt_path = parent.join("prompt.toml");
    let existing = if prompt_path.exists() {
        let prompt = PromptConfig::read(&prompt_path)?;
        fs::copy(&prompt_path, path)?;
        prompt
    } else {
        PromptConfig::default()
    };
    let mut prompt = PromptConfig::default();
    if let Ok(legacy) = fs::read_to_string(parent.join("settings")) {
        for line in legacy.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "prompt" if existing.left.is_none() && prompt.left.is_none() => {
                    prompt.left = Some(value.to_owned())
                }
                "prompt_hex" if existing.left.is_none() && prompt.left.is_none() => {
                    prompt.left = decode_prompt(value)
                }
                "rprompt" if existing.right.is_none() && prompt.right.is_none() => {
                    prompt.right = Some(value.to_owned())
                }
                "transient_prompt"
                    if existing.transient.is_none() && prompt.transient.is_none() =>
                {
                    prompt.transient = Some(value.to_owned())
                }
                "clock_format"
                    if existing.clock_format.is_none() && prompt.clock_format.is_none() =>
                {
                    prompt.clock_format = Some(value.to_owned())
                }
                "cursor_animation" => prompt.cursor_animation = legacy_bool(value),
                "syntax_highlighting" => prompt.syntax_highlighting = legacy_bool(value),
                "completion_enabled" => prompt.completion_enabled = legacy_bool(value),
                _ => {}
            }
        }
    }
    config::update(path, &prompt)
}

fn legacy_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}
