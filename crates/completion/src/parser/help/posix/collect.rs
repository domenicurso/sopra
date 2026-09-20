use super::super::option;
use super::collect_options::{next_value, option_specs};
use crate::graph::{CommandNode, PositionalSpec, Synopsis, ValueSpec};

pub(super) fn from_synopsis(synopsis: &Synopsis, root: &mut CommandNode, skip_first_literal: bool) {
    collect(synopsis, root, skip_first_literal, false, false, 0);
}

pub(super) fn options_from_synopsis(synopsis: &Synopsis, root: &mut CommandNode) {
    collect_options(synopsis, root);
}

fn collect_options(node: &Synopsis, root: &mut CommandNode) {
    match node {
        Synopsis::Sequence(nodes) => {
            for (index, node) in nodes.iter().enumerate() {
                if let Synopsis::Token(token) = node
                    && option::is_option_token(token)
                {
                    let mut specs = option_specs(token, nodes.get(index + 1));
                    let (value, _, attachment) = next_value(nodes.get(index + 1), &specs);
                    for spec in &mut specs {
                        spec.value = spec.value.take().or_else(|| value.clone());
                        spec.attachment = spec.attachment.merge(attachment);
                        root.merge_option(spec.clone());
                    }
                }
                collect_options(node, root);
            }
        }
        Synopsis::Optional(nodes) => {
            collect_options(&Synopsis::Sequence(nodes.clone()), root);
        }
        Synopsis::Repeat(inner) => collect_options(inner, root),
        Synopsis::Choice(alternatives) => {
            for alternative in alternatives {
                collect_options(&Synopsis::Sequence(alternative.clone()), root);
            }
        }
        Synopsis::Token(_) => {}
    }
}

fn collect(
    node: &Synopsis,
    root: &mut CommandNode,
    root_pending: bool,
    optional: bool,
    repeatable: bool,
    position_index: usize,
) -> usize {
    match node {
        Synopsis::Sequence(nodes) => collect_sequence(
            nodes,
            root,
            root_pending,
            optional,
            repeatable,
            position_index,
        ),
        Synopsis::Optional(nodes) => {
            collect_sequence(nodes, root, false, true, repeatable, position_index)
        }
        Synopsis::Choice(alternatives) => alternatives
            .iter()
            .map(|alternative| {
                collect(
                    &Synopsis::Sequence(alternative.clone()),
                    root,
                    false,
                    optional,
                    repeatable,
                    position_index,
                )
            })
            .max()
            .unwrap_or(0),
        Synopsis::Repeat(inner) => collect(inner, root, false, optional, true, position_index),
        Synopsis::Token(token) => {
            if root_pending && !token.starts_with('-') {
                return 0;
            }
            if option::is_option_token(token) {
                for spec in option_specs(token, None) {
                    root.merge_option(spec);
                }
                0
            } else {
                usize::from(merge_positional(
                    root,
                    token,
                    optional,
                    repeatable,
                    position_index,
                ))
            }
        }
    }
}

fn collect_sequence(
    nodes: &[Synopsis],
    root: &mut CommandNode,
    mut root_pending: bool,
    optional: bool,
    repeatable: bool,
    mut position_index: usize,
) -> usize {
    let start = position_index;
    let mut index = 0;
    while index < nodes.len() {
        let Synopsis::Token(token) = &nodes[index] else {
            position_index += collect(
                &nodes[index],
                root,
                false,
                optional,
                repeatable,
                position_index,
            );
            index += 1;
            continue;
        };
        if root_pending && !token.starts_with('-') {
            root_pending = false;
            index += 1;
            continue;
        }
        if option::is_option_token(token) {
            let next = nodes.get(index + 1);
            let mut specs = option_specs(token, next);
            let (value, consumed, attachment) = next_value(next, &specs);
            for spec in &mut specs {
                if spec.value.is_none() {
                    spec.value = value.clone();
                }
                spec.attachment = spec.attachment.merge(attachment);
                spec.repeatable |= repeatable;
                root.merge_option(spec.clone());
            }
            index += 1 + consumed;
            continue;
        }
        position_index += usize::from(merge_positional(
            root,
            token,
            optional,
            repeatable,
            position_index,
        ));
        index += 1;
    }
    position_index - start
}

fn merge_positional(
    root: &mut CommandNode,
    token: &str,
    optional: bool,
    repeatable: bool,
    position_index: usize,
) -> bool {
    if token == "..." || token == "--" || token.starts_with('=') || token.starts_with('{') {
        return false;
    }
    let name = token.trim_matches(['<', '>', '[', ']']);
    if name.is_empty() || option::synthetic_placeholder(name) {
        return false;
    }
    let value = if option::placeholder(token) || optional || repeatable {
        option::value_spec(name, optional)
    } else {
        ValueSpec::choice(name)
    };
    root.merge_positional_at(
        position_index,
        PositionalSpec {
            value,
            name: name.to_string(),
            description: None,
            optional,
            repeatable,
        },
    );
    true
}
