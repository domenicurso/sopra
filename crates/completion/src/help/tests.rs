use std::process::Command;

use super::parse_help;

#[test]
fn parses_help_from_installed_cli_families() {
    let cases = [
        ("bash", &["--help"][..]),
        ("cargo", &["--help"][..]),
        ("curl", &["--help"][..]),
        ("git", &["-h"][..]),
        ("npm", &["--help"][..]),
        ("python3", &["--help"][..]),
        ("rg", &["--help"][..]),
        ("rustc", &["--help"][..]),
        ("tar", &["--help"][..]),
        ("zsh", &["--help"][..]),
    ];
    let mut checked = 0;
    for (program, arguments) in cases {
        let Ok(output) = Command::new(program).args(arguments).output() else {
            continue;
        };
        let text = [output.stdout, output.stderr].concat();
        let text = String::from_utf8_lossy(&text);
        let spec = parse_help(&text);
        assert!(
            !spec.commands.is_empty() || !spec.options.is_empty() || !spec.positionals.is_empty(),
            "parser found no completion data in {program} help"
        );
        let shape_ok = match program {
            "cargo" | "npm" | "git" => !spec.commands.is_empty(),
            "python3" | "rg" => !spec.options.is_empty() && !spec.positionals.is_empty(),
            _ => !spec.options.is_empty(),
        };
        assert!(
            shape_ok,
            "parser missed the expected sections in {program} help"
        );
        match program {
            "git" => {
                assert!(spec.commands.iter().any(|command| command.name == "clone"));
                assert!(
                    spec.options
                        .iter()
                        .any(|option| option.names.iter().any(|name| name == "--version"))
                );
            }
            "npm" => {
                assert!(spec.commands.len() > 40);
                assert!(
                    spec.commands
                        .iter()
                        .any(|command| command.name == "install")
                );
            }
            "cargo" => assert!(spec.commands.iter().any(|command| command.name == "build")),
            "rg" => assert!(
                spec.options
                    .iter()
                    .any(|option| option.names.iter().any(|name| name == "--regexp"))
            ),
            "bash" => assert_option(&spec, "--version"),
            "curl" => assert_option(&spec, "--data"),
            "python3" => assert!(spec.positionals.iter().any(|item| item.name == "file")),
            "rustc" => assert_option(&spec, "--crate-type"),
            "tar" => assert_option(&spec, "--format"),
            "zsh" => assert_option(&spec, "--aliases"),
            _ => {}
        }
        checked += 1;
    }
    assert!(
        checked >= 5,
        "fewer than five CLI help sources were available"
    );
}

fn assert_option(spec: &super::HelpSpec, expected: &str) {
    assert!(
        spec.options
            .iter()
            .any(|option| option.names.iter().any(|name| name == expected)),
        "missing {expected}"
    );
}

#[test]
fn parses_subcommand_help_for_npm() {
    let Ok(output) = Command::new("npm").args(["install", "--help"]).output() else {
        return;
    };
    let text = [output.stdout, output.stderr].concat();
    let spec = parse_help(&String::from_utf8_lossy(&text));
    assert_option(&spec, "--install-strategy");

    let output = Command::new("npm")
        .args(["config", "--help"])
        .output()
        .expect("npm config help");
    let text = [output.stdout, output.stderr].concat();
    let spec = parse_help(&String::from_utf8_lossy(&text));
    assert!(spec.commands.iter().any(|command| command.name == "set"));
    assert!(spec.commands.iter().any(|command| command.name == "get"));
}
