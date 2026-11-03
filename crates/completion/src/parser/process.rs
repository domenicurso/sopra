use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, SyncSender},
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

const TIMEOUT: Duration = Duration::from_millis(600);

#[cfg(test)]
pub(crate) fn run(command: &mut Command) -> Option<String> {
    run_controlled(command, &super::control::Control::new(TIMEOUT), true).map(|output| output.0)
}

#[cfg(test)]
pub(crate) fn run_stdout(command: &mut Command) -> Option<String> {
    run_controlled(command, &super::control::Control::new(TIMEOUT), false).map(|output| output.0)
}

pub(crate) fn run_controlled(
    command: &mut Command,
    control: &super::control::Control,
    capture_stderr: bool,
) -> Option<(String, bool)> {
    if control.stopped() {
        return None;
    }
    let _permit = super::process_gate::Permit::acquire(control)?;
    apply_environment(command);
    isolate(command);
    command.stdin(Stdio::null()).stdout(Stdio::piped());
    command.stderr(if capture_stderr {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let stderr = capture_stderr.then(|| child.stderr.take()).flatten();
    let (events, receiver) = mpsc::sync_channel(32);
    let stdout_thread = reader(stdout, events.clone());
    let stderr_thread = stderr.map(|stream| reader(stream, events));
    let output = collect(
        &mut child,
        receiver,
        usize::from(capture_stderr) + 1,
        control,
    );
    // A timed-out probe has already been terminated. Do not wait on a reader
    // whose pipe may still be held by a descendant the probe did not create.
    output.as_ref()?;
    let _ = stdout_thread.join();
    if let Some(stderr_thread) = stderr_thread {
        let _ = stderr_thread.join();
    }
    output
}

fn apply_environment(command: &mut Command) {
    let overrides = command
        .get_envs()
        .map(|(key, value)| (key.to_owned(), value.map(std::ffi::OsStr::to_owned)))
        .collect::<Vec<_>>();
    command.env_clear().envs(sopra_shell_context::vars_os());
    for (key, value) in overrides {
        if let Some(value) = value {
            command.env(key, value);
        } else {
            command.env_remove(key);
        }
    }
}

fn collect(
    child: &mut Child,
    receiver: mpsc::Receiver<Event>,
    stream_count: usize,
    control: &super::control::Control,
) -> Option<(String, bool)> {
    let deadline = Instant::now() + TIMEOUT;
    let mut output = Vec::new();
    let mut streams = 0;
    while streams < stream_count {
        for event in receiver.try_iter().take(64) {
            match event {
                Event::Data(data) => {
                    if output.len() + data.len() > 1_000_000 {
                        terminate(child);
                        return None;
                    }
                    output.extend(data);
                }
                Event::Done => streams += 1,
            }
        }
        let finished = match child.try_wait() {
            Ok(status) => status.is_some(),
            Err(_) => {
                terminate(child);
                return None;
            }
        };
        if streams == stream_count {
            if !finished {
                terminate(child);
            }
            break;
        }
        if Instant::now() >= deadline || control.stopped() {
            terminate(child);
            return None;
        }
        thread::sleep(Duration::from_millis(2));
    }
    let success = child.wait().ok()?.success();
    (!output.is_empty()).then(|| {
        (
            super::normalize::terminal_text(&String::from_utf8_lossy(&output)),
            success,
        )
    })
}

#[cfg(unix)]
fn isolate(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn isolate(_command: &mut Command) {}

fn terminate(child: &mut Child) {
    #[cfg(unix)]
    {
        // The probe owns this process group, so a negative PID also terminates descendants.
        let _ = unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn reader<R: Read + Send + 'static>(
    mut input: R,
    events: SyncSender<Event>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match input.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(size) => {
                    if events.send(Event::Data(buffer[..size].to_vec())).is_err() {
                        return;
                    }
                }
            }
        }
        let _ = events.send(Event::Done);
    })
}

enum Event {
    Data(Vec<u8>),
    Done,
}

#[cfg(all(test, unix))]
#[path = "process_tests.rs"]
mod tests;
