use super::{Action, Context, Emphasis, Event, Part, Refresh, Text};
use std::{collections::BTreeMap, io, path::PathBuf, time::Duration};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Response {
    pub text: Text,
    pub action: Action,
    pub active: bool,
    pub interactive: bool,
    pub refresh: Refresh,
}

pub fn request(context: &Context, event: Option<Event>) -> String {
    format!(
        "W2\t{}\t{}\t{}\t{}",
        encode(&context.cwd.to_string_lossy()),
        encode(&pairs(&context.options)),
        encode(&pairs(&context.environment)),
        event.map_or(String::new(), Event::encode)
    )
}

pub fn read_request(line: &str) -> io::Result<(Context, Option<Event>)> {
    let fields = fields(line, 5)?;
    Ok((
        Context {
            cwd: PathBuf::from(decode(fields[1])?),
            options: read_pairs(&decode(fields[2])?)?,
            environment: read_pairs(&decode(fields[3])?)?,
        },
        (!fields[4].is_empty())
            .then(|| Event::decode(fields[4]))
            .transpose()?,
    ))
}

pub fn response(value: &Response) -> String {
    let refresh = match value.refresh {
        Refresh::OnContext => "-".into(),
        Refresh::Every(duration) => duration.as_millis().max(1).to_string(),
    };
    let action = match &value.action {
        Action::Ignored => "ignore".into(),
        Action::Consumed => "consume".into(),
        Action::ChangeDirectory(path) => format!("cd:{}", encode(&path.to_string_lossy())),
    };
    let parts = value
        .text
        .parts
        .iter()
        .map(|part| {
            format!(
                "{}:{}",
                match part.emphasis {
                    Emphasis::Normal => "n",
                    Emphasis::Dim => "d",
                    Emphasis::Bold => "b",
                    Emphasis::Selected => "s",
                },
                encode(&part.text)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "W2\t{}\t{}\t{refresh}\t{action}\t{parts}",
        u8::from(value.active),
        u8::from(value.interactive)
    )
}

pub fn read_response(line: &str) -> io::Result<Response> {
    let fields = fields(line, 6)?;
    let refresh = if fields[3] == "-" {
        Refresh::OnContext
    } else {
        Refresh::Every(Duration::from_millis(
            fields[3]
                .parse::<u64>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or_else(invalid)?,
        ))
    };
    let action = match fields[4] {
        "ignore" => Action::Ignored,
        "consume" => Action::Consumed,
        value => Action::ChangeDirectory(PathBuf::from(decode(
            value.strip_prefix("cd:").ok_or_else(invalid)?,
        )?)),
    };
    let parts = if fields[5].is_empty() {
        Vec::new()
    } else {
        fields[5]
            .split(',')
            .map(read_part)
            .collect::<io::Result<Vec<_>>>()?
    };
    Ok(Response {
        text: Text { parts },
        active: flag(fields[1])?,
        interactive: flag(fields[2])?,
        refresh,
        action,
    })
}

fn read_part(value: &str) -> io::Result<Part> {
    let (role, text) = value.split_once(':').ok_or_else(invalid)?;
    let text = decode(text)?;
    if text.chars().any(|ch| ch.is_control() && ch != '\n') {
        return Err(invalid());
    }
    let emphasis = match role {
        "n" => Emphasis::Normal,
        "d" => Emphasis::Dim,
        "b" => Emphasis::Bold,
        "s" => Emphasis::Selected,
        _ => return Err(invalid()),
    };
    Ok(Part { text, emphasis })
}

fn fields(line: &str, count: usize) -> io::Result<Vec<&str>> {
    let fields = line
        .trim_end_matches(['\r', '\n'])
        .split('\t')
        .collect::<Vec<_>>();
    if fields.len() != count || fields[0] != "W2" {
        return Err(invalid());
    }
    Ok(fields)
}

fn flag(value: &str) -> io::Result<bool> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(invalid()),
    }
}

fn encode(text: &str) -> String {
    text.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode(text: &str) -> io::Result<String> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(invalid());
    }
    let bytes = (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).map_err(|_| invalid()))
        .collect::<io::Result<Vec<_>>>()?;
    String::from_utf8(bytes).map_err(|_| invalid())
}

fn pairs(values: &BTreeMap<String, String>) -> String {
    values
        .iter()
        .map(|(key, value)| format!("{key}\0{value}\0"))
        .collect()
}

fn read_pairs(text: &str) -> io::Result<BTreeMap<String, String>> {
    let mut fields = text.split_terminator('\0');
    let mut output = BTreeMap::new();
    while let Some(name) = fields.next() {
        output.insert(name.into(), fields.next().ok_or_else(invalid)?.into());
    }
    Ok(output)
}

pub(super) fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid widget message")
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
