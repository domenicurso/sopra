use super::{debug_graph::Graph, help, help_pages};
use sopra_completion::inspection::{self, SourceFormat};
use std::{
    ffi::OsString,
    fs,
    io::{self, Read},
};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Command {
    Help,
    Generate(Graph),
    Sources(Graph),
    Cache(Graph),
    Json(Graph),
    Explain(Graph, String),
    Parse(CapturedSource),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CapturedSource {
    command: String,
    file: OsString,
    path: Vec<String>,
    format: SourceFormat,
}

pub(super) fn parse(arguments: &[OsString]) -> io::Result<Command> {
    let Some((action, rest)) = arguments.split_first() else {
        return Ok(Command::Help);
    };
    Ok(match action.to_str() {
        Some("--help" | "-h" | "help") if rest.is_empty() => Command::Help,
        Some("generate") => Command::Generate(Graph::parse(rest)?),
        Some("sources") => Command::Sources(Graph::parse(rest)?),
        Some("cache") => Command::Cache(Graph::parse(rest)?),
        Some("json") => Command::Json(Graph::parse(rest)?),
        Some("explain") => {
            let separator = rest
                .iter()
                .position(|word| word == "--")
                .ok_or_else(|| io::Error::other("use -- before the entry to explain"))?;
            let [entry] = &rest[separator + 1..] else {
                return Err(io::Error::other("supply one entry after --"));
            };
            Command::Explain(
                Graph::parse(&rest[..separator])?,
                entry
                    .to_str()
                    .ok_or_else(|| io::Error::other("entry must be UTF-8"))?
                    .into(),
            )
        }
        Some("parse") => Command::Parse(CapturedSource::parse(rest)?),
        _ => {
            return Err(io::Error::other(
                "unknown graph operation; run `sopra debug graph --help`",
            ));
        }
    })
}

pub(super) fn run(command: Command) -> io::Result<()> {
    match command {
        Command::Help => help::print(&help_pages::GRAPH),
        Command::Generate(graph) => graph.run(),
        Command::Sources(graph) => {
            super::output::print(&inspection::sources(&graph.query()).map_err(io::Error::other)?)
        }
        Command::Json(graph) => {
            super::output::print(&inspection::json(&graph.query()).map_err(io::Error::other)?)
        }
        Command::Explain(graph, entry) => super::output::print(
            &inspection::explain(&graph.query(), &entry).map_err(io::Error::other)?,
        ),
        Command::Cache(graph) => {
            super::output::print(&inspection::cache(&graph.query()).map_err(io::Error::other)?)
        }
        Command::Parse(parse) => parse.run(),
    }
}

impl CapturedSource {
    fn parse(arguments: &[OsString]) -> io::Result<Self> {
        let mut words = Vec::new();
        let mut format = SourceFormat::Help;
        for word in arguments {
            if word == "--man" {
                format = SourceFormat::Man;
            } else {
                words.push(word.clone());
            }
        }
        let [command, file, rest @ ..] = words.as_slice() else {
            return Err(io::Error::other(
                "supply a command name and source file, or - for stdin",
            ));
        };
        let command = command
            .to_str()
            .ok_or_else(|| io::Error::other("command name must be UTF-8"))?
            .into();
        let path = rest
            .iter()
            .map(|word| {
                word.to_str()
                    .map(str::to_string)
                    .ok_or_else(|| io::Error::other("command path must be UTF-8"))
            })
            .collect::<io::Result<_>>()?;
        Ok(Self {
            command,
            file: file.clone(),
            path,
            format,
        })
    }

    fn run(&self) -> io::Result<()> {
        let source = if self.file == "-" {
            let mut source = String::new();
            io::stdin().read_to_string(&mut source)?;
            source
        } else {
            fs::read_to_string(&self.file)?
        };
        let graph = inspection::parse_source(&self.command, &self.path, self.format, &source)
            .map_err(io::Error::other)?;
        super::output::print(&graph)
    }
}
