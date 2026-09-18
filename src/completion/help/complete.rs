use std::ops::Range;

use super::{
    Invocation,
    model::{ArgumentKind, HelpCommand, HelpOption, HelpSpec},
};
use crate::completion::{
    CompletionItem, CompletionKind, CompletionSource, Request, filesystem::FilesystemEngine,
};

pub(super) fn complete(
    spec: &HelpSpec,
    invocation: &Invocation,
    request: &Request,
    filesystem: &mut FilesystemEngine,
) -> Vec<CompletionItem> {
    if let Some(option) = pending_option(spec, invocation) {
        return value_items(option, request.replace.clone(), &invocation.active);
    }
    if invocation.active.starts_with('-') {
        return option_items(&spec.options, request.replace.clone());
    }
    if is_partial_subcommand(invocation) {
        return command_items(&spec.commands, request.replace.clone());
    }
    if let Some(command) = find_subcommand(spec, &invocation.args) {
        return positional_items(command, invocation, request, filesystem);
    }
    if let Some(items) = global_positional_items(spec, invocation, request, filesystem) {
        return items;
    }
    command_items(&spec.commands, request.replace.clone())
}

fn positional_items(
    command: &HelpCommand,
    invocation: &Invocation,
    request: &Request,
    filesystem: &mut FilesystemEngine,
) -> Vec<CompletionItem> {
    let Some(kind) = command.positional else {
        return Vec::new();
    };
    if !matches!(kind, ArgumentKind::File | ArgumentKind::Directory)
        || (kind == ArgumentKind::File && is_url(&invocation.active))
    {
        return Vec::new();
    }
    filesystem.complete(request, &invocation.active)
}

fn global_positional_items(
    spec: &HelpSpec,
    invocation: &Invocation,
    request: &Request,
    filesystem: &mut FilesystemEngine,
) -> Option<Vec<CompletionItem>> {
    let positional = spec.positionals.first()?;
    if !matches!(
        positional.kind,
        ArgumentKind::File | ArgumentKind::Directory
    ) || (positional.kind == ArgumentKind::File && is_url(&invocation.active))
    {
        return Some(Vec::new());
    }
    Some(filesystem.complete(request, &invocation.active))
}

fn find_subcommand<'a>(spec: &'a HelpSpec, args: &[String]) -> Option<&'a HelpCommand> {
    args.iter()
        .filter(|arg| !arg.starts_with('-'))
        .find_map(|arg| {
            spec.commands.iter().find(|command| {
                command.name == *arg || command.aliases.iter().any(|alias| alias == arg)
            })
        })
}

fn pending_option<'a>(spec: &'a HelpSpec, invocation: &Invocation) -> Option<&'a HelpOption> {
    let args = &invocation.args;
    let index = (0..args.len()).rev().find(|index| {
        spec.options
            .iter()
            .any(|option| option.names.iter().any(|name| name == &args[*index]))
    })?;
    let option = spec
        .options
        .iter()
        .find(|option| option.names.iter().any(|name| name == &args[index]))?;
    if !option.expects_value {
        return None;
    }
    let value_is_active = index + 2 == args.len() && !invocation.active.is_empty();
    let value_is_missing = index + 1 == args.len() && invocation.trailing_space;
    (value_is_active || value_is_missing).then_some(option)
}

fn is_partial_subcommand(invocation: &Invocation) -> bool {
    invocation.args.len() == 1
        && !invocation.trailing_space
        && invocation.args[0] == invocation.active
}

fn command_items(commands: &[HelpCommand], replace: Range<usize>) -> Vec<CompletionItem> {
    commands
        .iter()
        .flat_map(|command| {
            std::iter::once(command_item(&command.name, command, replace.clone())).chain(
                command
                    .aliases
                    .iter()
                    .map(|alias| command_item(alias, command, replace.clone())),
            )
        })
        .collect()
}

fn command_item(name: &str, command: &HelpCommand, replace: Range<usize>) -> CompletionItem {
    let mut item = CompletionItem::with_range(
        name,
        name,
        command.description.clone(),
        CompletionKind::Subcommand,
        replace,
        CompletionSource::CommandIndex,
    );
    item.group = Some("Commands".to_string());
    item
}

fn option_items(options: &[HelpOption], replace: Range<usize>) -> Vec<CompletionItem> {
    options
        .iter()
        .flat_map(|option| {
            option
                .names
                .iter()
                .map(|name| option_item(name, option, replace.clone()))
        })
        .collect()
}

fn option_item(name: &str, option: &HelpOption, replace: Range<usize>) -> CompletionItem {
    let mut item = CompletionItem::with_range(
        name,
        name,
        option.description.clone(),
        CompletionKind::Option,
        replace,
        CompletionSource::CommandIndex,
    );
    item.group = Some("Options".to_string());
    item
}

fn value_items(option: &HelpOption, replace: Range<usize>, query: &str) -> Vec<CompletionItem> {
    option
        .values
        .iter()
        .filter(|value| {
            query.is_empty()
                || value
                    .to_ascii_lowercase()
                    .contains(&query.to_ascii_lowercase())
        })
        .map(|value| {
            CompletionItem::with_range(
                value,
                value,
                None,
                CompletionKind::Value,
                replace.clone(),
                CompletionSource::CommandIndex,
            )
        })
        .collect()
}

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}
