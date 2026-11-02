use std::{env, ffi::OsString, fmt::Write, io, path::Path};

use super::{debug_graph::Graph, help, help_pages, output};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Command {
    Help,
    Syntax(String),
    Completions(Graph),
}

pub(super) fn parse(arguments: &[OsString]) -> io::Result<Command> {
    let Some((name, rest)) = arguments.split_first() else {
        return Ok(Command::Help);
    };
    Ok(match (name.to_str(), rest) {
        (Some("--help" | "-h" | "help"), []) => Command::Help,
        (Some("syntax" | "completions"), [flag]) if flag == "--help" || flag == "-h" => {
            Command::Help
        }
        (Some("syntax"), [source]) => Command::Syntax(
            source
                .to_str()
                .ok_or_else(|| io::Error::other("shell text must be UTF-8"))?
                .into(),
        ),
        (Some("completions"), rest) => Command::Completions(Graph::parse(rest)?),
        _ => {
            return Err(io::Error::other(
                "unknown debug command or arguments; run `sopra debug --help`",
            ));
        }
    })
}

pub(super) fn run(command: Command) -> io::Result<()> {
    match command {
        Command::Help => help::print(&help_pages::DEBUG),
        Command::Syntax(source) => output::print(&syntax_report(&source, &env::current_dir()?)),
        Command::Completions(graph) => graph.run(),
    }
}

fn syntax_report(source: &str, cwd: &Path) -> String {
    let mut report = String::from("Bytes\tKind\tColor\tText\n");
    for span in crate::syntax::highlight_without_cursor(source, cwd) {
        // Formatting into a String cannot fail.
        writeln!(
            report,
            "{}..{}\t{:?}\t{:?}\t{:?}",
            span.start,
            span.end,
            span.kind,
            span.style().fg.unwrap_or(ratatui::style::Color::Reset),
            &source[span.start..span.end]
        )
        .expect("writing to a String is infallible");
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_report_exposes_byte_ranges_and_escapes_multiline_tokens() {
        let source = "if echo 'café\nline' | /no-such-sopra-command; then echo -n hi; fi";
        let report = syntax_report(source, Path::new("."));
        assert!(report.contains("0..2\tKeyword\tYellow\t\"if\""));
        assert!(report.contains("RealCommand\tGreen\t\"echo\""));
        assert!(report.contains("String\tMagenta\t\"café\\nline\""));
        assert!(report.contains("Operator\tYellow\t\"|\""));
        assert!(report.contains("Flag\tBlue\t\"-n\""));
        assert!(report.contains("FakeCommand\tRed"));
        assert!(!report.contains('\x1b'));
    }
}
