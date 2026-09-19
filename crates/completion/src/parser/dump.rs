use std::path::Path;

use crate::{Request, commands, graph};

use super::discover::HelpRequest;

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
    let (_, executable) = commands::resolve(command, cwd)
        .ok_or_else(|| format!("cannot resolve executable: {command}"))?;
    let program = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command)
        .to_string();
    let request = HelpRequest {
        key: command.to_string(),
        command: executable,
        program,
        args: path,
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
    };
    let graph = if full {
        super::discover::resolve(&request)
    } else {
        super::discover::resolve_chunk(&request)
    }
    .ok_or_else(|| format!("no completion data found for: {command}"))?;
    Ok(graph::render(&graph))
}
