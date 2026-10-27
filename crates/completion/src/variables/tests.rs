use std::path::PathBuf;

use super::{complete, is_declaration_context, valid_name, visible_shell_variable};
use crate::Request;

fn request(line: &str) -> Request {
    Request {
        line: line.to_string(),
        cursor: line.len(),
        context_line: line.to_string(),
        context_cursor: line.len(),
        replace: 0..line.len(),
        cwd: PathBuf::from("."),
        generation: 1,
        context_key: String::new(),
    }
}

#[test]
fn declarations_allow_array_flags() {
    assert!(is_declaration_context("declare -a "));
    assert!(is_declaration_context("typeset -a "));
    assert!(!is_declaration_context("echo -a "));
}

#[test]
fn variable_names_use_shell_identifier_rules() {
    assert!(valid_name("HOME"));
    assert!(valid_name("_private2"));
    assert!(!valid_name("2FAST"));
    assert!(!valid_name("WITH-DASH"));
}

#[test]
fn public_sopra_runtime_settings_remain_visible() {
    for name in [
        "SOPRA_BIN",
        "SOPRA_PROMPT",
        "SOPRA_RPROMPT",
        "SOPRA_TRANSIENT_PROMPT",
        "SOPRA_CLOCK_FORMAT",
        "SOPRA_CURSOR_ANIMATION",
        "SOPRA_SYNTAX_HIGHLIGHTING",
        "SOPRA_COMPLETION_ENABLED",
    ] {
        assert!(visible_shell_variable(name), "{name} should be visible");
    }
}

#[test]
fn internal_sopra_variables_are_hidden() {
    for name in [
        "SOPRA_ALIASES",
        "SOPRA_FUNCTIONS",
        "SOPRA_COMMANDS",
        "SOPRA_VARIABLES",
        "SOPRA_ARRAYS",
        "SOPRA_REPOSITORY",
        "SOPRA_REF",
        "SOPRA_PREFIX",
        "SOPRA_REPO_ROOT",
        "_SOPRA_LOADED",
        "_SOPRA_RESULT_ACTION",
    ] {
        assert!(!visible_shell_variable(name), "{name} should be hidden");
    }
}

#[test]
fn unrelated_shell_variables_remain_visible() {
    assert!(visible_shell_variable("HOME"));
    assert!(visible_shell_variable("_private"));
}

#[test]
fn variables_are_completed_inside_double_quotes() {
    let line = "echo \"$HO";
    let request = request(line);
    let items = complete(&request, 6..line.len(), "$HO").expect("variable completion");
    assert!(items.iter().any(|item| item.insert == "$HOME"));
    assert!(items.iter().all(|item| item.replace == (6..line.len())));
}

#[test]
fn scalar_declarations_put_cursor_inside_quotes() {
    let line = "export HO";
    let request = request(line);
    let items = complete(&request, 7..line.len(), "HO").expect("declaration completion");
    let home = items
        .iter()
        .find(|item| item.insert == "HOME")
        .expect("HOME completion");
    assert_eq!(home.suffix, "=\"\"");
    assert_eq!(home.cursor_offset, Some(6));
}
