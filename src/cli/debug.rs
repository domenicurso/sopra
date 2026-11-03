use std::{env, ffi::OsString, io};

use super::{debug_graph::Graph, debug_syntax, help, help_pages, output};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Command {
    Help,
    Candidates(super::debug_candidates::Candidates),
    Providers,
    Graph(super::debug_graph_commands::Command),
    Syntax(String),
    Completions(Graph),
}

pub(super) fn parse(arguments: &[OsString]) -> io::Result<Command> {
    if arguments.first().is_some_and(|name| name == "graph") {
        return super::debug_graph_commands::parse(&arguments[1..]).map(Command::Graph);
    }
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
        (Some("candidates"), rest) => {
            Command::Candidates(super::debug_candidates::Candidates::parse(rest)?)
        }
        (Some("providers"), []) => Command::Providers,
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
        Command::Graph(command) => super::debug_graph_commands::run(command),
        Command::Help => help::print(&help_pages::DEBUG),
        Command::Syntax(source) => output::print(&debug_syntax::render(
            &source,
            &env::current_dir()?,
            &output::Output::detect(),
        )),
        Command::Completions(graph) => graph.run(),
        Command::Candidates(query) => query.run(),
        Command::Providers => output::print(
            &sopra_completion::CompletionEngine::provider_configuration()
                .map_err(io::Error::other)?,
        ),
    }
}
