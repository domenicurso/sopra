use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use super::{CompletionKind, Request, complete, kind_for_group, kind_for_match};
use crate::completion::model::normalize_description;

#[test]
fn zsh_groups_become_structured_kinds() {
    assert_eq!(kind_for_group(Some("options")), CompletionKind::Option);
    assert_eq!(
        kind_for_group(Some("directories")),
        CompletionKind::Directory
    );
    assert_eq!(kind_for_group(Some("unknown")), CompletionKind::Generic);
}

#[test]
fn default_groups_keep_file_options_and_positionals_distinct() {
    assert_eq!(
        kind_for_match(Some("-default-"), "--release", "cargo build ", false),
        CompletionKind::Option
    );
    assert_eq!(
        kind_for_match(Some("-default-"), "target", "cargo build ", false),
        CompletionKind::Positional
    );
    assert_eq!(
        kind_for_match(Some("-default-"), "README.md", "custom ", true),
        CompletionKind::File
    );
}

#[test]
fn zsh_display_descriptions_drop_the_repeated_match_label() {
    assert_eq!(
        normalize_description(
            "completion",
            Some(" completion  -- generate shell completion script\n".to_string()),
        ),
        Some("generate shell completion script".to_string())
    );
    assert_eq!(
        normalize_description("completion", Some("completion".to_string())),
        None
    );
    assert_eq!(
        normalize_description("completion", Some("completion:generate".to_string())),
        Some("generate".to_string())
    );
    assert_eq!(
        normalize_description("entry", Some("-- details".to_string())),
        Some("details".to_string())
    );
}

#[test]
fn zshrs_completes_a_command_option_without_a_shell_process() {
    let items = complete_request(Request {
        line: "git --vrsn".to_string(),
        cursor: 10,
        context_line: "git --".to_string(),
        context_cursor: 6,
        replace: 4..10,
        cwd: PathBuf::from("."),
        generation: 1,
        context_key: String::new(),
    });
    assert!(items.iter().any(|item| item.display == "--version"));
}

#[test]
fn zshrs_completes_command_arguments() {
    let items = complete_request(Request {
        line: "add-zsh-hook c".to_string(),
        cursor: 14,
        context_line: "add-zsh-hook ".to_string(),
        context_cursor: 13,
        replace: 13..14,
        cwd: PathBuf::from("."),
        generation: 1,
        context_key: String::new(),
    });
    assert!(items.iter().any(|item| item.display == "chpwd"));
    assert!(
        items
            .iter()
            .any(|item| item.kind == CompletionKind::Positional)
    );
}

fn complete_request(request: Request) -> Vec<super::super::Completion> {
    zsh::compsys::in_editor::bootstrap();
    let warm_deadline = Instant::now() + Duration::from_secs(2);
    while !zsh::compsys::in_editor::is_ready() && Instant::now() < warm_deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    let (mut items, mut incomplete) = complete(&request);
    let retry_deadline = Instant::now() + Duration::from_secs(2);
    while items.is_empty() && incomplete && Instant::now() < retry_deadline {
        std::thread::sleep(Duration::from_millis(25));
        (items, incomplete) = complete(&request);
    }
    items
}
