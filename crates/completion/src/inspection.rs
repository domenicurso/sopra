use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphMode {
    Root,
    Chunk,
    Recursive,
}

pub struct GraphQuery<'a> {
    pub command: &'a str,
    pub path: &'a [String],
    pub cwd: &'a Path,
    pub mode: GraphMode,
    pub refresh: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFormat {
    Help,
    Man,
}

pub fn generate(query: &GraphQuery<'_>) -> Result<String, String> {
    crate::parser::dump::generate(query)
}

pub fn sources(query: &GraphQuery<'_>) -> Result<String, String> {
    crate::parser::dump::sources(query)
}

pub fn cache(query: &GraphQuery<'_>) -> Result<String, String> {
    crate::parser::dump::cache(query)
}

pub fn parse_source(
    command: &str,
    path: &[String],
    format: SourceFormat,
    text: &str,
) -> Result<String, String> {
    let graph = match format {
        SourceFormat::Help => crate::parser::help::parse_for_with(text, command, path, true),
        SourceFormat::Man => crate::parser::man::parse_for_with(text, command, path, true),
    };
    if !graph.useful() {
        return Err("source contains no recognized completion data".into());
    }
    Ok(crate::graph::render(&graph))
}
