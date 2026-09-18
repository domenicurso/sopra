use super::{
    model::HelpCommand,
    parse_rows::Row,
    parse_values::{argument_kind, bracket_value, clean_description, placeholder},
};

pub(crate) fn parse_commands(rows: &[Row]) -> Vec<HelpCommand> {
    let root = common_root(rows);
    rows.iter()
        .filter_map(|row| {
            let (name, mut aliases) = command_name(&row.left, &root)?;
            aliases.extend(
                bracket_value(&row.description, "aliases:")
                    .into_iter()
                    .flat_map(|value| value.split([',', '|', ' ']))
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned),
            );
            Some(HelpCommand {
                name,
                description: clean_description(&row.description),
                aliases,
                positional: row
                    .left
                    .split_whitespace()
                    .find(|token| placeholder(token))
                    .map(argument_kind),
            })
        })
        .collect()
}

fn command_name(left: &str, root: &[String]) -> Option<(String, Vec<String>)> {
    let parts = left.split(',').map(str::trim).collect::<Vec<_>>();
    let first = command_token(parts.first().copied()?, root)?;
    let aliases = parts
        .iter()
        .skip(1)
        .filter_map(|part| command_token(part, &[]))
        .collect();
    Some((first, aliases))
}

fn command_token(value: &str, root: &[String]) -> Option<String> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    let root_matches = tokens.len() >= root.len()
        && tokens
            .iter()
            .take(root.len())
            .copied()
            .eq(root.iter().map(String::as_str));
    let start = usize::from(root_matches) * root.len();
    let token = tokens
        .get(start..)?
        .iter()
        .find(|token| !placeholder(token))?;
    let token = token.trim_matches(|character| matches!(character, '<' | '>' | '[' | ']' | ','));
    (token
        .chars()
        .any(|character| character.is_ascii_alphanumeric())
        && !token.starts_with('-'))
    .then(|| token.to_string())
}

fn common_root(rows: &[Row]) -> Vec<String> {
    let mut prefixes = rows.iter().map(|row| {
        row.left
            .split_whitespace()
            .take_while(|token| !placeholder(token))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    });
    let Some(mut root) = prefixes.next() else {
        return Vec::new();
    };
    for prefix in prefixes {
        root.truncate(
            root.iter()
                .zip(&prefix)
                .take_while(|(left, right)| left == right)
                .count(),
        );
    }
    if rows.len() == 1 {
        root.truncate(1);
    }
    root
}
