use std::{
    sync::mpsc::{Receiver, Sender},
    time::{Duration, Instant},
};

use super::{
    Completion, CompletionKind, CompletionResponse, CompletionResponseSource, CompletionSource,
    Request,
};

const COMPLETION_BUDGET: Duration = Duration::from_millis(120);
const COMPLETION_RETRY_BUDGET: Duration = Duration::from_secs(2);

pub(super) fn run(requests: Receiver<Request>, responses: Sender<CompletionResponse>) {
    zsh::compsys::in_editor::bootstrap();
    while !zsh::compsys::in_editor::is_ready() {
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut pending = None;
    loop {
        let mut request = match pending.take().or_else(|| requests.recv().ok()) {
            Some(request) => request,
            None => return,
        };
        while let Ok(next) = requests.try_recv() {
            request = next;
        }
        let (items, incomplete) = complete(&request);
        if !send_response(&responses, &request, items, incomplete) {
            return;
        }
        if incomplete {
            pending = retry_request(&requests, &responses, &request);
        }
    }
}

fn send_response(
    responses: &Sender<CompletionResponse>,
    request: &Request,
    items: Vec<Completion>,
    incomplete: bool,
) -> bool {
    responses
        .send(CompletionResponse {
            line: request.line.clone(),
            cursor: request.cursor,
            context_line: request.context_line.clone(),
            context_cursor: request.context_cursor,
            context_key: request.context_key.clone(),
            items,
            generation: request.generation,
            source: CompletionResponseSource::Zshrs,
            incomplete,
        })
        .is_ok()
}

fn retry_request(
    requests: &Receiver<Request>,
    responses: &Sender<CompletionResponse>,
    request: &Request,
) -> Option<Request> {
    let deadline = Instant::now() + COMPLETION_RETRY_BUDGET;
    while Instant::now() < deadline {
        if let Ok(mut next) = requests.try_recv() {
            while let Ok(newer) = requests.try_recv() {
                next = newer;
            }
            return Some(next);
        }
        std::thread::sleep(Duration::from_millis(25));
        let (items, incomplete) = complete(request);
        if !items.is_empty() || !incomplete {
            if !send_response(responses, request, items, incomplete) {
                return None;
            }
            return None;
        }
    }
    None
}

fn complete(request: &Request) -> (Vec<Completion>, bool) {
    let response = zsh::compsys::in_editor::complete_at(zsh::compsys::in_editor::CompsysRequest {
        line: &request.context_line,
        cursor: request.context_cursor,
        deadline: Instant::now() + COMPLETION_BUDGET,
        allow_exec: true,
    });
    let items = response
        .matches
        .into_iter()
        .filter_map(|item| completion_from_match(request, item))
        .collect();
    (items, response.is_incomplete)
}

fn completion_from_match(
    request: &Request,
    item: zsh::compsys::in_editor::CompsysMatch,
) -> Option<Completion> {
    if item.completion.is_empty() {
        return None;
    }
    let start = request
        .replace
        .start
        .saturating_add(item.replace_start)
        .min(request.replace.end);
    let mut completion = Completion::with_range(
        item.completion.clone(),
        item.completion,
        item.description,
        kind_for_group(item.group.as_deref()),
        start..request.replace.end,
        CompletionSource::Zshrs,
    );
    completion.group = item.group;
    Some(completion)
}

fn kind_for_group(group: Option<&str>) -> CompletionKind {
    match group.unwrap_or_default() {
        "commands" | "command" | "command_names" => CompletionKind::Command,
        "aliases" | "alias" => CompletionKind::Alias,
        "functions" | "function" => CompletionKind::Function,
        "builtins" | "builtin" => CompletionKind::Builtin,
        "options" | "option" => CompletionKind::Option,
        "subcommands" | "subcommand" => CompletionKind::Subcommand,
        "values" | "value" => CompletionKind::Value,
        "files" | "file" => CompletionKind::File,
        "directories" | "directory" => CompletionKind::Directory,
        "hosts" | "host" => CompletionKind::Host,
        "users" | "user" => CompletionKind::User,
        "arguments" | "positionals" | "positional" => CompletionKind::Positional,
        _ => CompletionKind::Generic,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };

    use super::{CompletionKind, Request, complete, kind_for_group};

    #[test]
    fn zsh_groups_become_structured_kinds() {
        assert_eq!(kind_for_group(Some("options")), CompletionKind::Option);
        assert_eq!(
            kind_for_group(Some("directories")),
            CompletionKind::Directory
        );
        assert_eq!(kind_for_group(Some("unknown")), CompletionKind::Generic);
    }

    #[test]
    fn zshrs_completes_a_command_option_without_a_shell_process() {
        zsh::compsys::in_editor::bootstrap();
        let warm_deadline = Instant::now() + Duration::from_secs(2);
        while !zsh::compsys::in_editor::is_ready() && Instant::now() < warm_deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        let request = Request {
            line: "git --vrsn".to_string(),
            cursor: 10,
            context_line: "git --".to_string(),
            context_cursor: 6,
            replace: 4..10,
            cwd: PathBuf::from("."),
            generation: 1,
            context_key: String::new(),
        };
        let (mut items, mut incomplete) = complete(&request);
        let retry_deadline = Instant::now() + Duration::from_secs(2);
        while items.is_empty() && incomplete && Instant::now() < retry_deadline {
            std::thread::sleep(Duration::from_millis(25));
            (items, incomplete) = complete(&request);
        }
        eprintln!(
            "zshrs option completion: {} items, incomplete={incomplete}, first={:?}",
            items.len(),
            items.first().map(|item| &item.display)
        );
        assert!(items.iter().any(|item| item.display == "--version"));
    }
}
