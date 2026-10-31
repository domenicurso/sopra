use std::{
    io::{self, Read, Write},
    os::fd::AsRawFd,
    process::{Child, ChildStderr, Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Default)]
pub(super) struct Parser {
    probe: Option<SyntaxProbe>,
    primary: Vec<u8>,
    incomplete: Option<String>,
}

impl Parser {
    pub(super) fn check(&mut self, buffer: &str) -> io::Result<bool> {
        let extension = self
            .incomplete
            .as_ref()
            .and_then(|previous| buffer.strip_prefix(previous))
            .and_then(|suffix| suffix.strip_prefix('\n'));
        if self.incomplete.is_some() && extension.is_none() {
            self.probe = None;
        }
        let code = extension.unwrap_or(buffer);
        if self.probe.is_none() {
            let mut probe = SyntaxProbe::start()?;
            self.primary = probe.prompt()?;
            self.probe = Some(probe);
        }
        let probe = self
            .probe
            .as_mut()
            .ok_or_else(|| io::Error::other("syntax checker unavailable"))?;
        let mut last = self.primary.clone();
        for line in code.split('\n') {
            let input = probe
                .child
                .stdin
                .as_mut()
                .ok_or_else(|| io::Error::other("syntax checker closed stdin"))?;
            writeln!(input, "{line}")?;
            input.flush()?;
            last = probe.prompt()?;
        }
        let incomplete = !last.ends_with(&self.primary);
        self.incomplete = incomplete.then(|| buffer.to_owned());
        Ok(incomplete)
    }
}

struct SyntaxProbe {
    child: Child,
    output: ChildStderr,
}

impl SyntaxProbe {
    fn start() -> io::Result<Self> {
        let mut child = Command::new("zsh")
            .args([
                "-dfni",
                "+m",
                "-o",
                "nozle",
                "-o",
                "nopromptcr",
                "-o",
                "nopromptsp",
            ])
            .env("LC_ALL", "C")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        let output = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("Zsh syntax checker did not provide stderr"))?;
        Ok(Self { child, output })
    }

    fn prompt(&mut self) -> io::Result<Vec<u8>> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut output = Vec::new();
        let mut bytes = [0; 4096];
        while Instant::now() < deadline {
            if output.ends_with(b"% ") || output.ends_with(b"# ") || output.ends_with(b"> ") {
                return Ok(output);
            }
            if self.poll(100)? {
                let count = self.output.read(&mut bytes)?;
                if count == 0 {
                    return Err(io::Error::other(
                        "Zsh syntax checker exited before prompting",
                    ));
                }
                output.extend_from_slice(&bytes[..count]);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Zsh syntax checker did not request another line",
        ))
    }

    fn poll(&self, timeout: i32) -> io::Result<bool> {
        let mut descriptor = libc::pollfd {
            fd: self.output.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // The descriptor borrows the live stderr pipe for this synchronous poll.
        let result = unsafe { libc::poll(&mut descriptor, 1, timeout) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                return Ok(false);
            }
            return Err(error);
        }
        Ok(result > 0)
    }
}

impl Drop for SyntaxProbe {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
