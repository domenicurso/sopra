use super::{cache::Source, discover::HelpRequest, help};
use std::collections::HashSet;

pub(super) fn arguments(request: &HelpRequest, sources: &[Source]) -> Vec<Vec<String>> {
    let mut flags = HashSet::new();
    for source in sources {
        let Source::Help(text) = source else {
            continue;
        };
        if !help::matches_path(text, &request.program, &request.args) {
            continue;
        }
        let graph = help::parse_for_with(text, &request.program, &request.args, false);
        for option in graph.root.options {
            if option.value.is_some() {
                continue;
            }
            if let Some(name) = option
                .names
                .iter()
                .find(|name| name.trim_start_matches('-').starts_with("help-"))
            {
                flags.insert(name.clone());
            }
        }
    }
    let mut flags = flags.into_iter().collect::<Vec<_>>();
    flags.sort();
    flags
        .into_iter()
        .take(8)
        .map(|flag| {
            let mut arguments = request.args.clone();
            arguments.push(flag);
            arguments
        })
        .collect()
}
