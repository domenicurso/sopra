use std::path::{Path, PathBuf};

pub(super) struct ParsedPath {
    pub(super) option_prefix: String,
    quote: Option<char>,
    close_quote: bool,
    pub(super) display_prefix: String,
    pub(super) base: PathBuf,
    pub(super) segments: Vec<String>,
    pub(super) leaf: String,
}

impl ParsedPath {
    pub(super) fn parse(token: &str, cwd: &Path) -> Option<Self> {
        let value_start = token
            .find('=')
            .filter(|_| token.starts_with('-'))
            .map_or(0, |index| index + 1);
        let option_prefix = token[..value_start].to_string();
        let mut raw = token[value_start..].to_string();
        let quote = raw.chars().next().filter(|char| matches!(char, '\'' | '"'));
        if quote.is_some() {
            raw.remove(0);
        }
        let close_quote = quote.map(|value| raw.ends_with(value)).unwrap_or(false);
        if close_quote {
            raw.pop();
        }
        let (base, display_prefix, remainder) = path_root(&raw, cwd)?;
        let trailing_separator = remainder.ends_with('/');
        let mut components = remainder.split('/').map(unescape).collect::<Vec<_>>();
        let leaf = if trailing_separator {
            while components.last().is_some_and(String::is_empty) {
                components.pop();
            }
            String::new()
        } else {
            components.pop().unwrap_or_default()
        };
        Some(Self {
            option_prefix,
            quote,
            close_quote,
            display_prefix,
            base,
            segments: components,
            leaf,
        })
    }

    pub(super) fn render(&self, segments: &[String], name: &str, directory: bool) -> String {
        let mut path = self.option_prefix.clone();
        if let Some(quote) = self.quote {
            path.push(quote);
        }
        path.push_str(&self.display_prefix);
        if !segments.is_empty() {
            if !path.is_empty()
                && !path.ends_with(['/', '='])
                && (self.quote.is_none() || !self.display_prefix.is_empty())
            {
                path.push('/');
            }
            path.push_str(
                &segments
                    .iter()
                    .map(|segment| escape(segment, self.quote))
                    .collect::<Vec<_>>()
                    .join("/"),
            );
            path.push('/');
        }
        path.push_str(&escape(name, self.quote));
        if directory {
            path.push('/');
        }
        if self.close_quote
            && let Some(quote) = self.quote
        {
            path.push(quote);
        }
        path
    }
}

fn path_root<'a>(path: &'a str, cwd: &Path) -> Option<(PathBuf, String, &'a str)> {
    if let Some(remainder) = path.strip_prefix('/') {
        return Some((PathBuf::from("/"), "/".to_string(), remainder));
    }
    if path == "~" || path.starts_with("~/") {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        return Some((
            home,
            "~/".to_string(),
            path.strip_prefix("~/").unwrap_or(""),
        ));
    }
    if let Some(remainder) = path.strip_prefix("./") {
        return Some((cwd.to_path_buf(), "./".to_string(), remainder));
    }
    Some((cwd.to_path_buf(), String::new(), path))
}

fn unescape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            output.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            output.push(character);
        }
    }
    if escaped {
        output.push('\\');
    }
    output
}

fn escape(value: &str, quote: Option<char>) -> String {
    match quote {
        Some('\'') => value.replace('\'', "'\\''"),
        Some('"') => value
            .chars()
            .flat_map(|character| match character {
                '\\' | '"' | '$' | '`' => vec!['\\', character],
                _ => vec![character],
            })
            .collect(),
        _ => value
            .chars()
            .flat_map(|character| match character {
                ' ' | '\t' | '\\' | '"' | '\'' | '$' | '`' | ';' | '&' | '|' | '<' | '>' => {
                    vec!['\\', character]
                }
                _ => vec![character],
            })
            .collect(),
    }
}

pub(super) fn hidden(name: &str, typed: &str) -> bool {
    name.starts_with('.') && !typed.starts_with('.')
}
