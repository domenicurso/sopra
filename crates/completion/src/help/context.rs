use std::path::{Path, PathBuf};

use super::HelpRequest;
use crate::{Request, commands, ranking};

#[derive(Debug, Clone)]
pub(crate) struct Invocation {
    pub(crate) key: String,
    pub(crate) command: Option<PathBuf>,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) active: String,
    pub(crate) trailing_space: bool,
}

pub(super) fn invocation(request: &Request) -> Option<Invocation> {
    let cursor = ranking::byte_offset(&request.line, request.cursor);
    let prefix = &request.line[..cursor];
    let (active_start, _) = ranking::token_range(&request.line, cursor);
    let parts = tokenize(prefix);
    let mut command = None;
    let mut args = Vec::new();
    for part in parts {
        if part.separator {
            command = None;
            args.clear();
        } else if command.is_none() {
            if is_assignment(&part.text) || is_wrapper(&part.text) {
                continue;
            }
            command = Some(part);
        } else {
            args.push(brush_parser::unquote_str(&part.text));
        }
    }
    let command = command?;
    if active_start <= command.end && !prefix[command.end..].chars().any(char::is_whitespace) {
        return None;
    }
    let program = brush_parser::unquote_str(&command.text);
    let (key, command_path) = resolve_command(&program, &request.cwd)?;
    let (_, active_end) = ranking::token_range(&request.line, cursor);
    let active = request.line[active_start..active_end.min(cursor)].to_string();
    Some(Invocation {
        key,
        command: command_path,
        program,
        args,
        active,
        trailing_space: prefix.chars().last().is_some_and(char::is_whitespace),
    })
}

pub(super) fn request_for(request: &Request, invocation: &Invocation) -> HelpRequest {
    HelpRequest {
        key: invocation.key.clone(),
        command: invocation.command.clone(),
        program: invocation.program.clone(),
        args: Vec::new(),
        cwd: request.cwd.clone(),
        request: request.clone(),
    }
}

fn resolve_command(program: &str, cwd: &Path) -> Option<(String, Option<PathBuf>)> {
    commands::resolve(program, cwd)
}

#[derive(Debug)]
struct ShellWord {
    text: String,
    end: usize,
    separator: bool,
}

fn tokenize(input: &str) -> Vec<ShellWord> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut start = None;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            start.get_or_insert(index);
            current.push(character);
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            current.push(character);
            if character == active_quote {
                quote = None;
            }
            continue;
        }
        if matches!(character, '\'' | '"') {
            start.get_or_insert(index);
            current.push(character);
            quote = Some(character);
        } else if character.is_whitespace() {
            flush_word(&mut words, &mut current, &mut start, index);
        } else if is_operator(character) {
            flush_word(&mut words, &mut current, &mut start, index);
            words.push(ShellWord {
                text: character.to_string(),
                end: index + character.len_utf8(),
                separator: true,
            });
        } else {
            start.get_or_insert(index);
            current.push(character);
        }
    }
    flush_word(&mut words, &mut current, &mut start, input.len());
    words
}

fn flush_word(
    words: &mut Vec<ShellWord>,
    current: &mut String,
    start: &mut Option<usize>,
    end: usize,
) {
    let Some(_start) = start.take() else {
        return;
    };
    words.push(ShellWord {
        text: std::mem::take(current),
        end,
        separator: false,
    });
}

fn is_operator(character: char) -> bool {
    matches!(character, '|' | '&' | ';' | '(' | ')' | '<' | '>')
}

fn is_assignment(word: &str) -> bool {
    word.find('=')
        .is_some_and(|index| index > 0 && !word[..index].contains('/'))
}

fn is_wrapper(word: &str) -> bool {
    matches!(
        brush_parser::unquote_str(word).as_str(),
        "builtin" | "command" | "env" | "exec" | "nohup" | "sudo" | "time"
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Request, invocation};

    #[test]
    fn finds_the_real_command_after_a_wrapper() {
        let request = Request {
            line: "command opencode import ".to_string(),
            cursor: 23,
            context_line: String::new(),
            context_cursor: 0,
            replace: 23..23,
            cwd: PathBuf::from("."),
            generation: 0,
            context_key: String::new(),
        };
        let _ = invocation(&request);
    }
}
