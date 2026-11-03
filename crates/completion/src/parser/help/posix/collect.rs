use super::super::option;
use super::collect_options::{next_value, option_specs};
use crate::graph::{CommandNode, PositionalSpec, Synopsis, ValueSpec};

#[derive(Clone, Copy, Default)]
struct Position {
    root_pending: bool,
    optional: bool,
    repeatable: bool,
    index: usize,
}

pub(super) fn from_synopsis(synopsis: &Synopsis, root: &mut CommandNode, skip_first_literal: bool) {
    collect(
        synopsis,
        root,
        Position {
            root_pending: skip_first_literal,
            ..Position::default()
        },
    );
}

pub(super) fn options_from_synopsis(node: &Synopsis, root: &mut CommandNode) {
    match node {
        Synopsis::Sequence(nodes) | Synopsis::Optional(nodes) => {
            for (index, node) in nodes.iter().enumerate() {
                if let Synopsis::Token(token) = node
                    && option::is_option_token(token)
                {
                    merge_options(root, token, nodes.get(index + 1), false);
                }
                options_from_synopsis(node, root);
            }
        }
        Synopsis::Repeat(inner) => options_from_synopsis(inner, root),
        Synopsis::Choice(alternatives) => {
            for alternative in alternatives {
                options_from_synopsis(&Synopsis::Sequence(alternative.clone()), root);
            }
        }
        Synopsis::Token(_) => {}
    }
}

fn collect(node: &Synopsis, root: &mut CommandNode, position: Position) -> usize {
    match node {
        Synopsis::Sequence(nodes) => sequence(nodes, root, position),
        Synopsis::Optional(nodes) => sequence(
            nodes,
            root,
            Position {
                optional: true,
                root_pending: false,
                ..position
            },
        ),
        Synopsis::Choice(alternatives) => alternatives
            .iter()
            .map(|alternative| {
                sequence(
                    alternative,
                    root,
                    Position {
                        root_pending: false,
                        ..position
                    },
                )
            })
            .max()
            .unwrap_or(0),
        Synopsis::Repeat(inner) => collect(
            inner,
            root,
            Position {
                repeatable: true,
                root_pending: false,
                ..position
            },
        ),
        Synopsis::Token(token) => {
            if position.root_pending && !token.starts_with('-') {
                return 0;
            }
            if option::is_option_token(token) {
                merge_options(root, token, None, position.repeatable);
                0
            } else {
                usize::from(merge_positional(root, token, position))
            }
        }
    }
}

fn sequence(nodes: &[Synopsis], root: &mut CommandNode, mut position: Position) -> usize {
    let start = position.index;
    let mut index = 0;
    while index < nodes.len() {
        let Synopsis::Token(token) = &nodes[index] else {
            position.index += collect(
                &nodes[index],
                root,
                Position {
                    root_pending: false,
                    ..position
                },
            );
            index += 1;
            continue;
        };
        if position.root_pending && !token.starts_with('-') {
            position.root_pending = false;
            index += 1;
            continue;
        }
        if option::is_option_token(token) {
            index += 1 + merge_options(root, token, nodes.get(index + 1), position.repeatable);
        } else {
            position.index += usize::from(merge_positional(root, token, position));
            index += 1;
        }
    }
    position.index - start
}

fn merge_options(
    root: &mut CommandNode,
    token: &str,
    next: Option<&Synopsis>,
    repeatable: bool,
) -> usize {
    let mut specs = option_specs(token, next);
    let (value, consumed, attachment) = next_value(next, &specs);
    let consumes = specs.iter().any(|spec| spec.value.is_none()) && value.is_some();
    for spec in &mut specs {
        if spec.value.is_none() && value.is_some() {
            spec.value = value.clone();
            spec.attachment = attachment;
        }
        spec.repeatable |= repeatable;
        root.merge_option(spec.clone());
    }
    if consumes { consumed } else { 0 }
}

fn merge_positional(root: &mut CommandNode, token: &str, position: Position) -> bool {
    if token == "..." || token == "--" || token.starts_with('=') || token.starts_with('{') {
        return false;
    }
    let name = token.trim_matches(['<', '>', '[', ']']);
    if name.is_empty() || option::synthetic_placeholder(name) {
        return false;
    }
    let value = if option::placeholder(token) || position.optional || position.repeatable {
        option::value_spec(name, position.optional)
    } else {
        ValueSpec::choice(name)
    };
    root.merge_positional_at(
        position.index,
        PositionalSpec {
            value,
            name: name.into(),
            description: None,
            optional: position.optional,
            repeatable: position.repeatable,
        },
    );
    true
}
