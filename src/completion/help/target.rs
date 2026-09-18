use super::{
    HelpRequest,
    context::Invocation,
    model::{HelpCommand, HelpSpec},
};
use crate::completion::Request;

pub(super) fn nested_request(
    request: &Request,
    invocation: &Invocation,
    spec: &HelpSpec,
) -> Option<(HelpRequest, Invocation)> {
    let path = command_path(spec, invocation);
    if path.is_empty() {
        return None;
    }
    let key = format!("{}\0{}", invocation.key, path.join("\0"));
    let mut nested = invocation.clone();
    nested.key = key.clone();
    nested.args = invocation.args[path.len()..].to_vec();
    Some((
        HelpRequest {
            key,
            command: invocation.command.clone(),
            program: invocation.program.clone(),
            args: path,
            cwd: request.cwd.clone(),
            request: request.clone(),
        },
        nested,
    ))
}

fn command_path(spec: &HelpSpec, invocation: &Invocation) -> Vec<String> {
    let mut path = Vec::new();
    for (index, argument) in invocation.args.iter().enumerate() {
        if !invocation.trailing_space
            && argument == &invocation.active
            && index + 1 == invocation.args.len()
        {
            break;
        }
        if argument.starts_with('-') {
            break;
        }
        if spec
            .commands
            .iter()
            .any(|command| matches_command(command, argument))
        {
            path.push(argument.clone());
        } else {
            break;
        }
    }
    path
}

fn matches_command(command: &HelpCommand, value: &str) -> bool {
    command.name == value || command.aliases.iter().any(|alias| alias == value)
}
