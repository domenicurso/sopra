use super::{
    ParsedOption,
    choices::simple_choice,
    names,
    values::{literal_value_spec, value_spec},
};
use crate::graph::ValueAttachment;

pub(crate) fn option_token(token: &str) -> Option<ParsedOption> {
    let token = token.trim_matches([',', '.']);
    if !token.starts_with('-') || token == "--" {
        return None;
    }
    if token.contains("[:") {
        return bracketed_value(token);
    }
    if let Some((name, value)) = token.split_once(':') {
        if !names::valid(name) || value.is_empty() {
            return None;
        }
        let mut spec = value_spec(value, false);
        if spec.choices.is_empty() && simple_choice(value) {
            spec.choices.push(value.to_string());
        }
        return Some(ParsedOption {
            name: format!("{name}:"),
            value: Some(spec),
            attachment: ValueAttachment::Attached,
        });
    }
    if token.contains(['[', '<']) {
        return bracketed_value(token);
    }
    if let Some((name, value)) = token.split_once('=') {
        if !names::valid(name) {
            return None;
        }
        let value = if simple_choice(value) {
            literal_value_spec(value)
        } else {
            value_spec(value, false)
        };
        return Some(ParsedOption {
            name: name.to_string(),
            value: Some(value),
            attachment: ValueAttachment::Attached,
        });
    }
    if !names::valid(token) {
        return None;
    }
    Some(ParsedOption {
        name: token.to_string(),
        value: None,
        attachment: ValueAttachment::Separate,
    })
}

fn bracketed_value(token: &str) -> Option<ParsedOption> {
    let start = token.find('[').or_else(|| token.find('<'))?;
    let name = token[..start].trim_end_matches('=').trim();
    if !names::valid(name) {
        return None;
    }
    let close = if token.as_bytes().get(start) == Some(&b'[') {
        ']'
    } else {
        '>'
    };
    let value = token[start + 1..].trim_end_matches(close);
    (!name.is_empty() && !value.is_empty()).then(|| {
        let attachment = if token[..start].ends_with('=')
            || value.starts_with(['=', ':'])
            || name.starts_with("--")
        {
            ValueAttachment::Attached
        } else {
            ValueAttachment::Either
        };
        ParsedOption {
            name: name.to_string(),
            value: Some(value_spec(
                value.trim_start_matches(['=', ':']),
                close == ']',
            )),
            attachment,
        }
    })
}
