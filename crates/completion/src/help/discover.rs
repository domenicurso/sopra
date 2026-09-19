use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant, UNIX_EPOCH},
};

use super::{HelpRequest, HelpSpec, output::clean_terminal_text, parse::parse_help};

const HELP_TIMEOUT: Duration = Duration::from_millis(600);
const OUTPUT_QUIET: Duration = Duration::from_millis(20);

pub(super) fn load(request: &HelpRequest) -> Option<HelpSpec> {
    if let Some(output) = read_cache(request) {
        let spec = parse_help(&output);
        if useful(&spec) {
            return Some(spec);
        }
    }
    for argument in ["--help", "-h"] {
        let Some(output) = run_program(request, argument) else {
            continue;
        };
        let spec = parse_help(&output);
        if useful(&spec) {
            write_cache(request, &output);
            return Some(spec);
        }
    }
    let output = run_man(request)?;
    let spec = parse_help(&output);
    if useful(&spec) {
        write_cache(request, &output);
    }
    useful(&spec).then_some(spec)
}

fn read_cache(request: &HelpRequest) -> Option<String> {
    let path = cache_path(request)?;
    let metadata = fs::metadata(&path).ok()?;
    (metadata.len() <= 1_000_000).then(|| fs::read_to_string(path).ok())?
}

fn write_cache(request: &HelpRequest, output: &str) {
    let Some(path) = cache_path(request) else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_ok() {
        let _ = fs::write(path, output);
    }
}

fn cache_path(request: &HelpRequest) -> Option<PathBuf> {
    let command = request.command.as_deref()?;
    let metadata = fs::metadata(command).ok()?;
    let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    let mut hasher = DefaultHasher::new();
    command.hash(&mut hasher);
    request.args.hash(&mut hasher);
    request.cwd.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    modified.as_nanos().hash(&mut hasher);
    Some(
        std::env::temp_dir()
            .join("sopra")
            .join("command-help")
            .join(format!("{:016x}.txt", hasher.finish())),
    )
}

fn run_program(request: &HelpRequest, argument: &str) -> Option<String> {
    let program = request
        .command
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new(&request.program));
    let mut command = Command::new(program);
    command
        .args(&request.args)
        .arg(argument)
        .current_dir(&request.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_command(&mut command)
}

fn run_man(request: &HelpRequest) -> Option<String> {
    run_command(
        Command::new("man")
            .args(["-P", "cat", &request.program])
            .current_dir(&request.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    )
}

fn run_command(command: &mut Command) -> Option<String> {
    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let stderr = child.stderr.take()?;
    let (events, event_rx) = mpsc::channel();
    let stdout_thread = spawn_reader(stdout, events.clone());
    let stderr_thread = spawn_reader(stderr, events);
    let deadline = Instant::now() + HELP_TIMEOUT;
    let mut output = Vec::new();
    let mut completed_streams = 0;
    let mut last_output = None;
    while completed_streams < 2 {
        while let Ok(event) = event_rx.try_recv() {
            match event {
                OutputEvent::Data(data) => {
                    output.extend(data);
                    last_output = Some(Instant::now());
                }
                OutputEvent::Done => completed_streams += 1,
            }
        }
        let finished = child.try_wait().ok()?.is_some();
        let quiet = last_output.is_some_and(|started| started.elapsed() >= OUTPUT_QUIET);
        if (finished || quiet) && !output.is_empty() {
            if !finished {
                let _ = child.kill();
            }
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return None;
        }
        thread::sleep(Duration::from_millis(2));
    }
    let _ = child.wait();
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    while let Ok(event) = event_rx.try_recv() {
        if let OutputEvent::Data(data) = event {
            output.extend(data);
        }
    }
    Some(clean_terminal_text(&String::from_utf8_lossy(&output)))
}

enum OutputEvent {
    Data(Vec<u8>),
    Done,
}

fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    events: Sender<OutputEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(size) => {
                    if events
                        .send(OutputEvent::Data(buffer[..size].to_vec()))
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
        let _ = events.send(OutputEvent::Done);
    })
}

fn useful(spec: &HelpSpec) -> bool {
    !spec.commands.is_empty() || !spec.options.is_empty() || !spec.positionals.is_empty()
}
