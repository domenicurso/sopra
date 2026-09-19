use super::parse_rows::Row;

pub(crate) fn rows(text: &str) -> Vec<Row> {
    let mut usage = String::new();
    let mut collecting = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.to_ascii_lowercase().starts_with("usage:") {
            collecting = true;
            usage.push_str(trimmed);
            usage.push(' ');
        } else if collecting && !trimmed.is_empty() && line.starts_with([' ', '\t']) {
            usage.push_str(trimmed);
            usage.push(' ');
        } else if collecting {
            break;
        }
    }
    option_names(&usage)
        .into_iter()
        .map(|name| Row {
            left: name,
            description: String::new(),
        })
        .collect()
}

pub(crate) fn command_rows(text: &str) -> Vec<Row> {
    let Some(root) = usage_root(text) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    let mut collecting = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.to_ascii_lowercase().starts_with("usage:") {
            collecting = true;
            continue;
        }
        if !collecting || trimmed.is_empty() {
            continue;
        }
        if !trimmed
            .split_whitespace()
            .next()
            .is_some_and(|word| word == root)
        {
            if !line.starts_with([' ', '\t']) {
                break;
            }
            continue;
        }
        let (left, description) = columns(trimmed);
        let tokens = left.split_whitespace().skip(1);
        if tokens
            .filter(|token| !token.starts_with('-'))
            .any(|token| !token.starts_with(['<', '[']))
        {
            rows.push(Row { left, description });
        }
    }
    if rows.len() > 1 { rows } else { Vec::new() }
}

fn usage_root(text: &str) -> Option<&str> {
    let mut after_usage = false;
    for line in text.lines().map(str::trim_start) {
        if let Some(rest) = line
            .strip_prefix("Usage:")
            .or_else(|| line.strip_prefix("usage:"))
        {
            after_usage = true;
            if let Some(root) = rest.split_whitespace().next() {
                return Some(root);
            }
        } else if after_usage && !line.is_empty() {
            return line.split_whitespace().next();
        }
    }
    None
}

fn columns(line: &str) -> (String, String) {
    let mut spaces = 0;
    for (index, character) in line.char_indices() {
        if character == ' ' {
            spaces += 1;
        } else if spaces >= 2 {
            return (
                line[..index - spaces].to_string(),
                line[index..].trim().to_string(),
            );
        } else {
            spaces = 0;
        }
    }
    (line.trim().to_string(), String::new())
}

fn option_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (index, character) in text.char_indices() {
        if character != '-' || text[..index].ends_with('-') {
            continue;
        }
        let Some(next) = text[index + 1..].chars().next() else {
            continue;
        };
        if !next.is_ascii_alphanumeric() && next != '-' {
            continue;
        }
        let end = text[index + 1..]
            .char_indices()
            .find(|(_, value)| !value.is_ascii_alphanumeric() && *value != '-')
            .map_or(text.len(), |(offset, _)| index + 1 + offset);
        let name = text[index..end].to_string();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}
