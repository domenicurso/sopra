use std::path::Path;

use crate::{Request, commands, graph};

use super::discover::{CachePolicy, HelpRequest};
use crate::inspection::{GraphMode, GraphQuery};

pub(crate) fn render_command_graph(command: &str, cwd: &Path) -> Result<String, String> {
    render(command, cwd, Vec::new(), true)
}

pub(crate) fn render_command_root(command: &str, cwd: &Path) -> Result<String, String> {
    render(command, cwd, Vec::new(), false)
}

pub(crate) fn render_command_chunk(
    command: &str,
    cwd: &Path,
    path: &[String],
) -> Result<String, String> {
    render(command, cwd, path.to_vec(), false)
}

fn render(command: &str, cwd: &Path, path: Vec<String>, full: bool) -> Result<String, String> {
    generate(&GraphQuery {
        command,
        cwd,
        path: &path,
        mode: if full {
            GraphMode::Recursive
        } else {
            GraphMode::Chunk
        },
        refresh: false,
    })
}

pub(crate) fn request(query: &GraphQuery<'_>) -> Result<HelpRequest, String> {
    if query.mode == GraphMode::Root && !query.path.is_empty() {
        return Err("root inspection does not accept a nested path".into());
    }
    let GraphQuery {
        command, cwd, path, ..
    } = query;
    let (_, executable) = commands::resolve(command, cwd)
        .ok_or_else(|| format!("cannot resolve executable: {command}"))?;
    let program = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command)
        .to_string();
    Ok(HelpRequest {
        control: super::control::Control::new(std::time::Duration::from_secs(15)),
        cache: if query.refresh {
            CachePolicy::Refresh
        } else {
            CachePolicy::Use
        },
        environment: sopra_shell_context::Snapshot::capture(),
        key: command.to_string(),
        command: executable,
        program,
        args: path.to_vec(),
        cwd: cwd.to_path_buf(),
        request: Request {
            line: command.to_string(),
            cursor: command.chars().count(),
            context_line: command.to_string(),
            context_cursor: command.chars().count(),
            replace: command.len()..command.len(),
            cwd: cwd.to_path_buf(),
            generation: 0,
            context_key: command.to_string(),
        },
        discover_usage_commands: true,
    })
}

pub(crate) fn generate(query: &GraphQuery<'_>) -> Result<String, String> {
    let request = request(query)?;
    let graph = if query.mode == GraphMode::Recursive {
        super::discover::resolve(&request)
    } else {
        super::discover::resolve_chunk(&request)
    }
    .ok_or_else(|| format!("no completion data found for: {}", query.command))?;
    Ok(graph::render(&graph))
}

pub(crate) fn sources(query: &GraphQuery<'_>) -> Result<String, String> {
    use std::fmt::Write;
    let request = request(query)?;
    let cached = (!query.refresh)
        .then(|| super::cache::read(&request))
        .flatten();
    let hit = cached.is_some();
    let sources = cached.unwrap_or_else(|| {
        let sources = super::discover::probes::sources(&request);
        super::cache::write(&request, &sources);
        sources
    });
    let mut output = format!(
        "Executable: {}\nDirectory: {}\nCache: {}\n",
        request
            .command
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| query.command.into()),
        query.cwd.display(),
        if hit { "hit" } else { "probed" }
    );
    for (index, source) in sources.iter().enumerate() {
        let (kind, text) = match source {
            super::cache::Source::Help(text) => ("help", text),
            super::cache::Source::Man(text) => ("man", text),
        };
        let _ = writeln!(
            output,
            "\n=== {kind} source {} ({} bytes) ===\n{text}",
            index + 1,
            text.len()
        );
    }
    Ok(output)
}

pub(crate) fn cache(query: &GraphQuery<'_>) -> Result<String, String> {
    let request = request(query)?;
    Ok(format!(
        "{}\n{}",
        super::cache::inspect(&request),
        super::snapshot::inspect(&request)
    ))
}
