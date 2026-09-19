mod commands;

use super::{Document, option};
use crate::graph::CommandNode;

pub(crate) fn parse(document: &Document, root: &mut CommandNode, path: &[String]) {
    for row in &document.options {
        if let Some(option) = option::option_row(&row.left, &row.description) {
            root.merge_option(option);
        }
    }
    let choices = document
        .positionals
        .iter()
        .flat_map(|row| commands::choice_names(&row.left))
        .collect::<Vec<_>>();
    for row in &document.positionals {
        if let Some(command) = commands::positional_command(row, &choices) {
            root.merge_subcommand(command);
            continue;
        }
        if let Some(positional) = commands::positional(row) {
            root.merge_positional(positional);
        }
    }
    for row in &document.commands {
        if let Some(command) = commands::parse(row, root, path) {
            root.merge_subcommand(command);
        }
    }
}
