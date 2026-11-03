use super::{cache::Cache, config, execute, query::Query};
use crate::{Request, parser::control::Control};
use std::{fs, path::Path, time::Duration};

fn query(cwd: &Path, text: &str) -> Query {
    Query {
        request: Request {
            line: text.into(),
            cursor: text.chars().count(),
            context_line: text.into(),
            context_cursor: text.chars().count(),
            replace: 0..text.len(),
            cwd: cwd.into(),
            generation: 1,
            context_key: String::new(),
        },
        command: "fixture".into(),
        path: vec!["run".into()],
        slots: vec![config::Slot::Positional(0)],
        query: text.into(),
        replace: 0..text.len(),
        environment: sopra_shell_context::Snapshot::capture(),
    }
}

#[test]
fn recipes_validate_source_slot_and_runtime_limits() {
    for recipe in [
        "[[provider]]\nid='x'\ncommand='fixture'\nexec=[]\npositional=0",
        "[[provider]]\nid='x'\ncommand='fixture'\nexec=['sh']\npositional=-1",
        "[[provider]]\nid='x'\ncommand='fixture'\nexec=['sh']\npositional=0\ntimeout_ms=501",
        "[[provider]]\nid='x'\ncommand='fixture'\nexec=['sh']\npositional=0\noption='--mode'",
        "[[provider]]\nid='x'\ncommand='fixture'\nexec=['sh']\npositional=0\ntt1_ms=100",
    ] {
        assert!(config::parse(recipe).is_err(), "{recipe}");
    }
    assert!(config::parse("").expect("empty override file").is_empty());
    assert_eq!(
        config::parse(include_str!("builtin.toml"))
            .expect("valid bundled recipes")
            .len(),
        5
    );
}

#[test]
fn json_keys_refresh_after_file_changes_and_scope_to_the_matching_slot()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("sopra-provider-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join("package.json"),
        r#"{"scripts":{"first":"echo one"}}"#,
    )?;
    let recipes = config::parse(
        "[[provider]]\nid='scripts'\ncommand='fixture'\npath=['run']\npositional=0\njson_file='package.json'\npointer='/scripts'\nroot_marker='package.json'\n",
    )?;
    let query = query(&directory, "");
    let control = Control::new(Duration::from_secs(2));
    let mut cache = Cache::new();
    assert_eq!(
        execute::resolve_with(&query, &recipes, &control, &mut cache)?,
        Some(vec!["first".into()])
    );
    fs::write(
        directory.join("package.json"),
        r#"{"scripts":{"second-long-name":"echo two"}}"#,
    )?;
    assert_eq!(
        execute::resolve_with(&query, &recipes, &control, &mut cache)?,
        Some(vec!["second-long-name".into()])
    );
    let mut unrelated = query.clone();
    unrelated.path = vec!["other".into()];
    assert_eq!(
        execute::resolve_with(&unrelated, &recipes, &control, &mut cache)?,
        None
    );
    fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn executable_providers_reject_failure_and_timeouts_without_caching_diagnostics()
-> Result<(), String> {
    let query = query(Path::new("/tmp"), "");
    let mut cache = Cache::new();
    for script in ["printf diagnostic; exit 1", "sleep 2; printf late"] {
        let recipe = format!(
            "[[provider]]\nid='exec'\ncommand='fixture'\npath=['run']\npositional=0\nexec=['sh','-c','{script}']\ntimeout_ms=20\n"
        );
        let providers = config::parse(&recipe)?;
        assert!(
            execute::resolve_with(
                &query,
                &providers,
                &Control::new(Duration::from_secs(1)),
                &mut cache
            )
            .is_err()
        );
    }
    let providers = config::parse(
        "[[provider]]\nid='exec'\ncommand='fixture'\npath=['run']\npositional=0\nexec=['sh','-c','printf \"alpha\\nbeta\\n\"']\n",
    )?;
    assert_eq!(
        execute::resolve_with(
            &query,
            &providers,
            &Control::new(Duration::from_secs(1)),
            &mut cache
        )?,
        Some(vec!["alpha".into(), "beta".into()])
    );
    Ok(())
}

#[test]
fn live_values_escape_shell_metacharacters_and_keep_display_names() {
    let items = super::items(
        &query(Path::new("/tmp"), ""),
        vec!["a b;$(echo bad)".into()],
    );
    assert_eq!(items[0].display, "a b;$(echo bad)");
    assert_eq!(items[0].insert, "a\\ b\\;\\$\\(echo\\ bad\\)");
}
