use super::{
    model::HelpSpec,
    parse_commands::parse_commands,
    parse_rows::rows,
    parse_values::{parse_options, parse_positionals},
};

pub(crate) fn parse_help(text: &str) -> HelpSpec {
    let (commands, options, positionals) = rows(text);
    HelpSpec {
        commands: parse_commands(&commands),
        options: parse_options(&options),
        positionals: parse_positionals(&positionals),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_help;

    const HELP: &str = r#"
Commands:
  opencode completion          generate shell completion script
  opencode [project]           start opencode tui                                          [default]
  opencode attach <url>        attach to a running opencode server
  opencode run [message..]     run opencode with a message
  opencode import <file>       import session data from JSON file or URL
  opencode plugin <module>     install plugin and update config                      [aliases: plug]

Positionals:
  project  path to start opencode in                                                        [string]

Options:
  -h, --help          show help                                                            [boolean]
      --log-level     log level                 [string] [choices: "DEBUG", "INFO", "WARN", "ERROR"]
      --port          port to listen on                                        [number] [default: 0]
"#;

    #[test]
    fn parses_commands_options_choices_and_argument_kinds() {
        let spec = parse_help(HELP);
        assert!(spec.commands.iter().any(|command| command.name == "import"));
        assert_eq!(
            spec.commands
                .iter()
                .find(|command| command.name == "import")
                .and_then(|command| command.positional),
            Some(super::super::model::ArgumentKind::File)
        );
        assert!(
            spec.commands
                .iter()
                .any(|command| command.aliases == ["plug"])
        );
        let option = spec
            .options
            .iter()
            .find(|option| option.names.contains(&"--log-level".to_string()))
            .expect("log-level option");
        assert_eq!(option.values, ["DEBUG", "INFO", "WARN", "ERROR"]);
        assert!(option.expects_value);
        assert_eq!(
            spec.positionals[0].kind,
            super::super::model::ArgumentKind::Directory
        );
    }
}
