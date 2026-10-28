use std::{
    io::{self, Read, Write},
    os::fd::AsRawFd,
    process::{Child, ChildStderr, Command, Stdio},
    time::{Duration, Instant},
};

// An interactive noexec shell still requests PS2 for incomplete syntax, including
// heredocs and escaped newlines. Keeping stdin open distinguishes that request
// from the recovery prompt Zsh prints after an unexpected EOF.
pub(super) fn needs_continuation(buffer: &str) -> io::Result<bool> {
    let mut parser = SyntaxProbe::start()?;
    let primary = parser.prompt()?;
    let mut last = primary.clone();
    for line in buffer.split('\n') {
        let input = parser.child.stdin.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Zsh syntax checker closed stdin")
        })?;
        writeln!(input, "{line}")?;
        input.flush()?;
        last = parser.prompt()?;
    }
    Ok(!last.ends_with(&primary))
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
            let ready = self.poll(if output.is_empty() { 100 } else { 10 })?;
            if !ready
                && (output.ends_with(b"% ") || output.ends_with(b"# ") || output.ends_with(b"> "))
            {
                return Ok(output);
            }
            if ready {
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

#[cfg(test)]
mod tests {
    use super::needs_continuation;

    #[test]
    fn zsh_distinguishes_continuations_from_complete_and_invalid_commands() -> std::io::Result<()> {
        for code in [
            "echo \\",
            "echo 'hello",
            "for x in a; do",
            "echo hi |",
            "cat <<EOF",
            "cat <<EOF\nhello",
        ] {
            assert!(needs_continuation(code)?, "{code:?}");
        }
        for code in [
            "echo hello",
            "echo )",
            "for x in a; do\necho $x\ndone",
            "cat <<EOF\nhello\nEOF",
            "echo \\\nhello",
        ] {
            assert!(!needs_continuation(code)?, "{code:?}");
        }
        Ok(())
    }

    #[test]
    fn syntax_check_does_not_execute_expansions_or_commands() -> std::io::Result<()> {
        let file = std::env::temp_dir().join(format!("sopra-noexec-{}", std::process::id()));
        let code = format!(
            "echo $(touch '{}'); touch '{}'",
            file.display(),
            file.display()
        );
        assert!(!needs_continuation(&code)?);
        assert!(!file.exists());
        Ok(())
    }
}
