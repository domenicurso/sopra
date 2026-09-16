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
        let mut components = remainder.split('/').map(unescape).collect::<Vec<_>>();
        let leaf = if remainder.ends_with('/') {
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
        let mut path = format!("{}{}", self.option_prefix, self.quote.unwrap_or_default());
        path.push_str(&self.display_prefix);
        if !segments.is_empty() {
            if !path.ends_with('/') {
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
        if self.close_quote {
            path.push(self.quote.unwrap_or_default());
        }
        path
    }
}

fn path_root<'a>(path: &'a str, cwd: &Path) -> Option<(PathBuf, String, &'a str)> {
    if path.starts_with('/') {
        return Some((PathBuf::from("/"), "/".to_string(), &path[1..]));
    }
    if path == "~" || path.starts_with("~/") {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        return Some((
            home,
            "~/".to_string(),
            path.strip_prefix("~/").unwrap_or(""),
        ));
    }
    if path.starts_with("./") {
        return Some((cwd.to_path_buf(), "./".to_string(), &path[2..]));
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
