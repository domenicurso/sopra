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
    for key in SettingKey::ALL {
        settings.save(&file, key).expect("settings should save");
    }
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
    let config = directory.join("config.toml");
    settings.save(&config, SettingKey::RightPrompt)?;
    let text = fs::read_to_string(&config)?;
    settings.save(&config, SettingKey::CursorAnimation)?;
    assert!(fs::read_to_string(&config)?.contains("cursor_animation = true"));
    assert!(text.contains("# preserved") && text.contains("# keep"));
    let saved = crate::prompt::config::PromptConfig::read(&config)?;
    assert_eq!(saved.right.as_deref(), Some("[cyan]{clock}[/]"));
    fs::remove_dir_all(directory)
}

#[test]
fn migration_keeps_legacy_values_and_comments() -> std::io::Result<()> {
    let directory = env::temp_dir().join(format!("sopra-migrate-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join("prompt.toml"),
        "# keep\n[prompt]\nleft = 'custom'\n",
    )?;
    fs::write(
        directory.join("settings"),
        "prompt=old\nrprompt=right\ncursor_animation=0\n",
    )?;
    let path = directory.join("config.toml");
    let mut settings = Settings::load(Some(&path));
    settings.syntax_highlighting = false;
    settings.save(&path, SettingKey::SyntaxHighlighting)?;
    let config = crate::prompt::config::PromptConfig::read(&path)?;
    assert_eq!(config.left.as_deref(), Some("custom"));
    assert_eq!(config.right.as_deref(), Some("right"));
    assert_eq!(config.cursor_animation, Some(false));
    assert_eq!(config.syntax_highlighting, Some(false));
    let migrated = fs::read_to_string(&path)?;
    assert!(migrated.contains("# keep") && migrated.contains("left = 'custom'"));
    assert!(directory.join("prompt.toml").exists() && directory.join("settings").exists());
    fs::remove_dir_all(directory)
}
