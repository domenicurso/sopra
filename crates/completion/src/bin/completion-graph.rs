use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let mode = match args.first().map(String::as_str) {
        Some("--root") => {
            args.remove(0);
            "root"
        }
        Some("--chunk") => {
            args.remove(0);
            "chunk"
        }
        _ => "full",
    };
    let cwd = take_cwd(&mut args);
    let Some(command) = args.first().cloned() else {
        eprintln!("usage: completion-graph [--root|--chunk] <command> [path ...] [--cwd dir]");
        return ExitCode::FAILURE;
    };
    let result = match mode {
        "root" => sopra_completion::dump_command_root(&command, &cwd),
        "chunk" => sopra_completion::dump_command_chunk(&command, &args[1..], &cwd),
        _ => sopra_completion::dump_command_graph(&command, &cwd),
    };
    match result {
        Ok(graph) => {
            print!("{graph}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn take_cwd(args: &mut Vec<String>) -> PathBuf {
    let Some(index) = args.iter().position(|value| value == "--cwd") else {
        return env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    };
    let cwd = args.get(index + 1).map(PathBuf::from);
    args.drain(index..=index + usize::from(cwd.is_some()));
    cwd.unwrap_or_else(|| PathBuf::from("."))
}
