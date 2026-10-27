use std::{env, fs};

use super::{SettingKey, Settings, path};

#[test]
fn settings_round_trip_preserves_schema_values() {
    let file = env::temp_dir().join(format!("sopra-settings-{}", std::process::id()));
    let settings = Settings {
        prompt: "%n in %~\n$ ".to_string(),
        rprompt: "right".to_string(),
        transient_prompt: "done $ ".to_string(),
        clock_format: "%H[dim]:[/]%M".to_string(),
        cursor_animation: false,
        syntax_highlighting: false,
        completion_enabled: true,
    };
    let _ = fs::remove_file(&file);
    settings
        .save(&file, SettingKey::Prompt)
        .expect("settings should save");
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

#[test]
fn saving_settings_updates_existing_prompt_toml() -> std::io::Result<()> {
    let directory = env::temp_dir().join(format!("sopra-config-sync-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    let file = directory.join("prompt.toml");
    fs::write(&file, "# preserved\n[prompt]\nright = 'old' # keep\n")?;
    let settings = Settings {
        rprompt: "[cyan]{clock}[/]".to_string(),
        ..Settings::default()
    };
    settings.save(&directory.join("settings"), SettingKey::RightPrompt)?;
    let first = fs::read_to_string(&file)?;
    settings.save(&directory.join("settings"), SettingKey::CursorAnimation)?;
    assert_eq!(fs::read_to_string(&file)?, first);
    let text = first;
    assert!(text.contains("# preserved") && text.contains("# keep"));
    let saved = crate::prompt::config::PromptConfig::read(&file)?;
    assert_eq!(saved.right.as_deref(), Some("[cyan]{clock}[/]"));
    fs::remove_dir_all(directory)
}
