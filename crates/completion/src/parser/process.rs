use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_millis(600);

pub(crate) fn run(command: &mut Command) -> Option<String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let stderr = child.stderr.take()?;
    let (events, receiver) = mpsc::channel();
    let stdout_thread = reader(stdout, events.clone());
    let stderr_thread = reader(stderr, events);
    let output = collect(&mut child, receiver);
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    output
}

fn collect(child: &mut Child, receiver: mpsc::Receiver<Event>) -> Option<String> {
    let deadline = Instant::now() + TIMEOUT;
    let mut output = Vec::new();
    let mut streams = 0;
    while streams < 2 {
        while let Ok(event) = receiver.try_recv() {
            match event {
                Event::Data(data) => output.extend(data),
                Event::Done => streams += 1,
            }
        }
        let finished = child.try_wait().ok()?.is_some();
        if streams == 2 {
            if !finished {
                let _ = child.kill();
            }
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(Duration::from_millis(2));
    }
    let _ = child.wait();
    (!output.is_empty()).then(|| super::normalize::terminal_text(&String::from_utf8_lossy(&output)))
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
