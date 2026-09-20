use std::ops::Range;

use crate::{
    Request,
    graph::{CommandNode, OptionSpec, PositionalSpec, ValueSpec},
    parser::Invocation,
};

pub(super) fn pending_value<'a>(
    node: &'a CommandNode,
    invocation: &Invocation,
    request: &Request,
) -> Option<(&'a ValueSpec, String, Range<usize>)> {
    let args = &invocation.args;
    let current = args.last().map(String::as_str).unwrap_or_default();
    if let Some((name, query)) = current.split_once('=') {
        let option = option_named(node, name)?;
        let value = option.value.as_ref()?;
        let query_start = name.len() + 1;
        return Some((
            value,
            query.to_string(),
            request.replace.start + query_start..request.replace.end,
        ));
    }
    if let Some((option, query_start)) = attached_option(node, current) {
        let value = option.value.as_ref()?;
        return Some((
            value,
            current[query_start..].to_string(),
            request.replace.start + query_start..request.replace.end,
        ));
    }
    let index = (0..args.len())
        .rev()
        .find(|index| option_named(node, &args[*index]).is_some())?;
    let option = option_named(node, &args[index])?;
    let value = option.value.as_ref()?;
    if index + 1 == args.len() && invocation.trailing_space {
        return Some((value, String::new(), request.replace.clone()));
    }
    if index + 2 == args.len() && !invocation.active.is_empty() {
        return Some((value, invocation.active.clone(), request.replace.clone()));
    }
    None
}

pub(super) fn option_named<'a>(node: &'a CommandNode, name: &str) -> Option<&'a OptionSpec> {
    node.options
        .iter()
        .find(|option| option.names.iter().any(|candidate| candidate == name))
}

pub(super) fn next_positional<'a>(
    node: &'a CommandNode,
    invocation: &Invocation,
) -> Option<&'a PositionalSpec> {
    let mut index = 0;
    let mut skip_value = false;
    for (position, argument) in invocation.args.iter().enumerate() {
        if !invocation.trailing_space
            && position + 1 == invocation.args.len()
            && argument == &invocation.active
        {
            break;
        }
        if skip_value {
            skip_value = false;
            continue;
        }
        if let Some(option) = option_named(node, argument) {
            skip_value = option.value.is_some();
        } else if attached_option(node, argument).is_none() && !argument.starts_with('-') {
            index += 1;
        }
    }
    let positional = node
        .positionals
        .get(index)
        .or_else(|| node.positionals.last())?;
    (index == 0 || positional.repeatable || index < node.positionals.len()).then_some(positional)
}

fn attached_option<'a>(node: &'a CommandNode, token: &str) -> Option<(&'a OptionSpec, usize)> {
    node.options.iter().find_map(|option| {
        if option.value.is_none() || option.attachment == crate::graph::ValueAttachment::Separate {
            return None;
        }
        option.names.iter().find_map(|name| {
            let suffix = token.strip_prefix(name)?;
            if suffix.is_empty() || name.starts_with("--") && !suffix.starts_with('=') {
                return None;
            }
            let offset = name.len() + usize::from(suffix.starts_with('='));
            (offset < token.len()).then_some((option, offset))
        })
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::pending_value;
    use crate::{
        Request,
        graph::{CommandNode, OptionSpec, ValueAttachment, ValueSpec},
        parser::Invocation,
    };

    #[test]
    fn attached_short_values_use_only_the_value_as_the_query() {
        let node = CommandNode {
            options: vec![OptionSpec {
                names: vec!["-C".to_string()],
                description: None,
                value: Some(ValueSpec::named("num")),
                attachment: ValueAttachment::Either,
                repeatable: false,
            }],
            ..CommandNode::named("grep")
        };
        let invocation = Invocation {
            key: "grep".to_string(),
            command: None,
            program: "grep".to_string(),
            args: vec!["-C5".to_string()],
            active: "-C5".to_string(),
            trailing_space: false,
        };
        let request = Request {
            line: "grep -C5".to_string(),
            cursor: 8,
            context_line: String::new(),
            context_cursor: 0,
            replace: 5..8,
            cwd: PathBuf::from("."),
            generation: 0,
            context_key: String::new(),
        };
        let (_, query, replace) = pending_value(&node, &invocation, &request).expect("value");
        assert_eq!(query, "5");
        assert_eq!(replace, 7..8);
    }
}
