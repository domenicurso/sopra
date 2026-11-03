use crate::parser::{cache::Source, discover::HelpRequest, process};
use std::{
    process::{Command, Stdio},
    thread,
};

pub(crate) fn sources(request: &HelpRequest) -> Vec<Source> {
    let mut probes = Vec::new();
    for args in help_arguments(request) {
        let request = request.clone();
        probes.push(thread::spawn(move || {
            request
                .environment
                .with(|| run_program(&request, &args))
                .map(Source::Help)
        }));
    }
    let man_request = request.clone();
    probes.push(thread::spawn(move || {
        man_request
            .environment
            .with(|| run_man(&man_request))
            .map(Source::Man)
    }));
    let mut sources = probes
        .into_iter()
        .filter_map(|probe| probe.join().ok().flatten())
        .collect::<Vec<_>>();
    let extra = crate::parser::extra_help::arguments(request, &sources);
    let extra = thread::scope(|scope| {
        extra
            .iter()
            .map(|arguments| {
                scope.spawn(|| {
                    request
                        .environment
                        .with(|| run_program(request, arguments))
                        .map(Source::Help)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|probe| probe.join().ok().flatten())
            .collect::<Vec<_>>()
    });
    sources.extend(extra);
    let mut unique = Vec::new();
    for source in sources {
        if !unique.contains(&source) {
            unique.push(source);
        }
    }
    unique
}

fn help_arguments(request: &HelpRequest) -> Vec<Vec<String>> {
    ["--help", "-h"]
        .iter()
        .map(|probe| {
            let mut args = request.args.clone();
            args.push((*probe).to_string());
            args
        })
        .chain((!request.args.is_empty()).then(|| {
            let mut args = vec!["help".to_string()];
            args.extend(request.args.iter().cloned());
            args
        }))
        .collect()
}

fn run_program(request: &HelpRequest, args: &[String]) -> Option<String> {
    let program = request
        .command
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new(&request.program));
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(&request.cwd)
        .env("TERM", "dumb")
        .env("NO_COLOR", "1")
        .env("PAGER", "cat")
        .env("GIT_PAGER", "cat")
        .env_remove("GIT_MAN_VIEWER")
        .env("MANPAGER", "cat")
        .env("LC_ALL", "C");
    process::run(&mut command)
}

fn run_man(request: &HelpRequest) -> Option<String> {
    let title = std::iter::once(request.program.as_str())
        .chain(request.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("-");
    let mut command = Command::new("man");
    command
        .args(["-P", "cat", &title])
        .current_dir(&request.cwd)
        .env("MANPAGER", "cat")
        .env("PAGER", "cat")
        .stdin(Stdio::null());
    process::run_stdout(&mut command)
}
