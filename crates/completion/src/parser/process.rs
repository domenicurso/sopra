use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

const TIMEOUT: Duration = Duration::from_millis(600);

pub(crate) fn run(command: &mut Command) -> Option<String> {
    run_with_stderr(command, true)
}

pub(crate) fn run_stdout(command: &mut Command) -> Option<String> {
    run_with_stderr(command, false)
}

fn run_with_stderr(command: &mut Command, capture_stderr: bool) -> Option<String> {
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
    let (events, receiver) = mpsc::channel();
    let stdout_thread = reader(stdout, events.clone());
    let stderr_thread = stderr.map(|stream| reader(stream, events));
    let output = collect(&mut child, receiver, usize::from(capture_stderr) + 1);
    // A timed-out probe has already been terminated. Do not wait on a reader
    // whose pipe may still be held by a descendant the probe did not create.
    output.as_ref()?;
    let _ = stdout_thread.join();
    if let Some(stderr_thread) = stderr_thread {
        let _ = stderr_thread.join();
    }
    output
}

fn collect(
    child: &mut Child,
    receiver: mpsc::Receiver<Event>,
    stream_count: usize,
) -> Option<String> {
    let deadline = Instant::now() + TIMEOUT;
    let mut output = Vec::new();
    let mut streams = 0;
    while streams < stream_count {
        while let Ok(event) = receiver.try_recv() {
            match event {
                Event::Data(data) => output.extend(data),
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
        if Instant::now() >= deadline {
            terminate(child);
            return None;
        }
        thread::sleep(Duration::from_millis(2));
    }
    let _ = child.wait();
    (!output.is_empty()).then(|| super::normalize::terminal_text(&String::from_utf8_lossy(&output)))
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

fn reader<R: Read + Send + 'static>(mut input: R, events: Sender<Event>) -> thread::JoinHandle<()> {
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
mod tests {
    use std::{process::Command, time::Instant};

    use super::run;

    #[test]
    fn timeout_terminates_descendants_that_hold_output_pipes() {
        let started = Instant::now();
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10 & wait"]);

        assert!(run(&mut command).is_none());
        assert!(started.elapsed().as_secs() < 2);
    }

    #[test]
    fn stdout_probe_excludes_diagnostics() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf stdout; printf stderr >&2"]);

        assert_eq!(super::run_stdout(&mut command).as_deref(), Some("stdout"));
    }
}
