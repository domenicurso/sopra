use super::config::{Provider, Slot};
use crate::{Request, engine::arguments, graph::CommandNode, parser::Invocation};
use std::{
    ops::Range,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub(crate) struct Query {
    pub(crate) request: Request,
    pub(super) command: String,
    pub(super) path: Vec<String>,
    pub(super) slots: Vec<Slot>,
    pub(super) query: String,
    pub(super) replace: Range<usize>,
    pub(super) environment: sopra_shell_context::Snapshot,
}

pub(crate) fn plan(
    request: &Request,
    invocation: &Invocation,
    path: Vec<String>,
    node: &CommandNode,
) -> Option<Query> {
    let (slots, query, replace) = if let Some((value, query, replace)) =
        arguments::pending_value(node, invocation, request)
    {
        let option = node.options.iter().find(|option| {
            option
                .value
                .as_ref()
                .is_some_and(|candidate| std::ptr::eq(value, candidate))
        })?;
        (
            option.names.iter().cloned().map(Slot::Option).collect(),
            query,
            replace,
        )
    } else {
        if invocation.active.starts_with('-') {
            return None;
        }
        (
            vec![Slot::Positional(arguments::position_index(
                node, invocation,
            ))],
            invocation.active.clone(),
            request.replace.clone(),
        )
    };
    Some(Query {
        request: request.clone(),
        command: Path::new(&invocation.program).file_name()?.to_str()?.into(),
        path,
        slots,
        query,
        replace,
        environment: sopra_shell_context::Snapshot::capture(),
    })
}

impl Query {
    pub(super) fn matches(&self, provider: &Provider) -> bool {
        self.command == provider.command
            && self.path == provider.path
            && self.slots.contains(&provider.slot)
    }
    pub(super) fn root(&self, provider: &Provider) -> Option<PathBuf> {
        let Some(marker) = &provider.marker else {
            return Some(self.request.cwd.clone());
        };
        self.request
            .cwd
            .ancestors()
            .find(|directory| directory.join(marker).exists())
            .map(Path::to_path_buf)
    }
}
