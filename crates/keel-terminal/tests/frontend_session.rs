use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock, mpsc},
    thread,
    time::{Duration, Instant},
};

use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};
use serial_test::serial;

static BUILD_GUARD: Mutex<()> = Mutex::new(());
static BUILD_RESULT: OnceLock<Result<(), String>> = OnceLock::new();

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root should resolve")
}

fn ensure_frontend_artifacts() -> io::Result<()> {
    let _guard = BUILD_GUARD.lock().expect("build guard lock should succeed");
    let result = BUILD_RESULT.get_or_init(|| {
        let status = Command::new(repo_root().join("scripts/build-zsh-module.sh"))
            .current_dir(repo_root())
            .status()
            .map_err(|error| format!("failed to launch build script: {error}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("build script exited with status {status}"))
        }
    });

    result
        .as_ref()
        .map_err(|message| io::Error::other(message.clone()))?;
    Ok(())
}

struct FrontendSession {
    child: Box<dyn Child + Send>,
    writer: Box<dyn Write + Send>,
    receiver: mpsc::Receiver<Vec<u8>>,
    parser: vt100::Parser,
    transcript: Vec<u8>,
}

impl FrontendSession {
    fn start() -> io::Result<Self> {
        ensure_frontend_artifacts()?;

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 30,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| io::Error::other(format!("openpty failed: {error}")))?;

        let repo_root = repo_root();
        let mut command = CommandBuilder::new("bash");
        command.arg("-lc");
        command.arg(format!(
            "cd '{}' && KEEL_MINIMAL_SHELL=1 KEEL_BUILD_ON_START=0 TERM=xterm-256color ./scripts/start-keel-session.sh",
            repo_root.display()
        ));

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| io::Error::other(format!("spawn failed: {error}")))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| io::Error::other(format!("reader clone failed: {error}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| io::Error::other(format!("writer take failed: {error}")))?;

        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || drain_reader(reader, sender));

        Ok(Self {
            child,
            writer,
            receiver,
            parser: vt100::Parser::new(30, 120, 0),
            transcript: Vec::new(),
        })
    }

    fn send_line(&mut self, line: &str) -> io::Result<()> {
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\r")?;
        self.writer.flush()
    }

    fn wait_for_prompt(&mut self) -> io::Result<()> {
        self.wait_until(|screen, transcript| {
            screen.contains("keel> ") || transcript.contains("keel> ")
        })
    }

    fn wait_until<F>(&mut self, predicate: F) -> io::Result<()>
    where
        F: Fn(&str, &str) -> bool,
    {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let screen = self.screen_contents();
            let transcript = String::from_utf8_lossy(&self.transcript);
            if predicate(&screen, &transcript) {
                return Ok(());
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("timed out waiting for session state\nscreen:\n{screen}\n\ntranscript:\n{transcript}"),
                ));
            }

            let remaining = deadline.saturating_duration_since(now);
            match self
                .receiver
                .recv_timeout(remaining.min(Duration::from_millis(100)))
            {
                Ok(chunk) => {
                    self.transcript.extend_from_slice(&chunk);
                    self.parser.process(&chunk);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!(
                            "session exited before reaching expected state\nscreen:\n{screen}\n\ntranscript:\n{transcript}"
                        ),
                    ));
                }
            }
        }
    }

    fn screen_contents(&self) -> String {
        self.parser.screen().contents()
    }

    fn visible_lines(&self) -> Vec<String> {
        self.screen_contents()
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.trim().is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }
}

impl Drop for FrontendSession {
    fn drop(&mut self) {
        let _ = self.send_line("exit");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn drain_reader(mut reader: Box<dyn Read + Send>, sender: mpsc::Sender<Vec<u8>>) {
    let mut buffer = [0u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if sender.send(buffer[..read].to_vec()).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

#[test]
#[serial]
fn renders_command_output_above_the_next_prompt() -> io::Result<()> {
    let mut session = FrontendSession::start()?;
    session.wait_for_prompt()?;
    session.send_line(r#"echo "hello""#)?;
    session.wait_until(|_, transcript| transcript.contains("hello\r\n\u{1b}[?25l"))?;

    assert!(
        String::from_utf8_lossy(&session.transcript).contains("hello\r\n\u{1b}[?25l"),
        "frontend should re-enter only after command output newline\ntranscript:\n{}",
        String::from_utf8_lossy(&session.transcript)
    );
    Ok(())
}

#[test]
#[serial]
fn restores_prompt_after_an_alternate_screen_style_command() -> io::Result<()> {
    let mut session = FrontendSession::start()?;
    session.wait_for_prompt()?;
    session.send_line(r#"printf '\033[?1049hALT\033[?1049l'"#)?;
    session.wait_until(|screen, transcript| {
        screen.contains("keel> ")
            && transcript.contains("?1049h")
            && transcript.contains("?1049l")
    })?;

    let lines = session.visible_lines();
    assert!(
        lines.last().is_some_and(|line| line.contains("keel>")),
        "active prompt should resume as the last visible line: {lines:?}"
    );
    assert!(
        lines.iter().filter(|line| line.contains("keel>")).count() >= 1,
        "prompt should remain visible after alternate-screen command: {lines:?}"
    );
    Ok(())
}
