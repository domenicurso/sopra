use crate::{
    Request,
    graph::{CommandNode, OptionSpec, PositionalSpec, ValueSpec},
    parser::Invocation,
};
use std::ops::Range;

pub(crate) fn pending_value<'a>(
    node: &'a CommandNode,
    invocation: &Invocation,
    request: &Request,
) -> Option<(&'a ValueSpec, String, Range<usize>)> {
    let current = if invocation.trailing_space {
        ""
    } else {
        invocation.active.as_str()
    };
    if invocation
        .args
        .iter()
        .take(completed_count(invocation))
        .any(|arg| arg == "--")
    {
        return None;
    }
    if let Some((option, offset)) = attached_option(node, current) {
        return Some((
            option.value.as_ref()?,
            current[offset..].to_string(),
            request.replace.start + offset..request.replace.end,
        ));
    }
    let state = scan(node, invocation);
    let option = state.pending?;
    if current.starts_with('-')
        && node
            .options
            .iter()
            .any(|option| option.names.iter().any(|name| name.starts_with(current)))
    {
        return None;
    }
    Some((
        option.value.as_ref()?,
        current.to_string(),
        request.replace.clone(),
    ))
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
    let state = scan(node, invocation);
    node.positionals
        .get(state.positionals)
        .or_else(|| node.positionals.last().filter(|last| last.repeatable))
}

pub(super) fn completed_count(invocation: &Invocation) -> usize {
    invocation
        .args
        .len()
        .saturating_sub(usize::from(!invocation.trailing_space))
}

pub(super) fn selected<'a>(node: &'a CommandNode, invocation: &Invocation) -> Vec<&'a OptionSpec> {
    scan(node, invocation).selected
}

struct Scan<'a> {
    pending: Option<&'a OptionSpec>,
    positionals: usize,
    selected: Vec<&'a OptionSpec>,
    terminated: bool,
}

fn scan<'a>(node: &'a CommandNode, invocation: &Invocation) -> Scan<'a> {
    let mut state = Scan {
        pending: None,
        positionals: 0,
        selected: Vec::new(),
        terminated: false,
    };
    for argument in invocation.args.iter().take(completed_count(invocation)) {
        if state.pending.take().is_some()
            && argument != "--"
            && option_named(node, argument).is_none()
        {
            continue;
        }
        if state.terminated {
            state.positionals += 1;
            continue;
        }
        if argument == "--" {
            state.terminated = true;
            continue;
        }
        let option = option_named(node, argument)
            .or_else(|| attached_option(node, argument).map(|(option, _)| option));
        if let Some(option) = option {
            state.selected.push(option);
            if option_named(node, argument).is_some()
                && option.attachment.separate()
                && option.value.is_some()
            {
                state.pending = Some(option);
            }
        } else if !argument.starts_with('-') {
            state.positionals += 1;
        }
    }
    state
}

fn attached_option<'a>(node: &'a CommandNode, token: &str) -> Option<(&'a OptionSpec, usize)> {
    node.options.iter().find_map(|option| {
        option.value.as_ref()?;
        option.names.iter().find_map(|name| {
            option
                .attachment
                .value_start(name, token)
                .map(|offset| (option, offset))
        })
    })
}

pub(crate) fn position_index(node: &CommandNode, invocation: &Invocation) -> usize {
    scan(node, invocation).positionals
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
