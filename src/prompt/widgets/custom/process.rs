use std::{
    io::{self, BufReader},
    os::fd::AsRawFd,
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use super::{
    sdk::{decode, encode},
    transport,
};

#[derive(Default)]
pub(super) struct Output {
    pub text: String,
    pub active: bool,
    pub interactive: bool,
    pub directory: Option<std::path::PathBuf>,
}

pub(super) struct Process {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Process {
    pub(super) fn start(binary: &Path) -> io::Result<Self> {
        let mut child = Command::new(binary)
            .env_clear()
            .envs(sopra_shell_context::vars_os())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("missing widget stdin"))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("missing widget stdout"))?;
        transport::nonblocking(input.as_raw_fd())?;
        transport::nonblocking(output.as_raw_fd())?;
        Ok(Self {
            child,
            input,
            output: BufReader::new(output),
        })
    }

    pub(super) fn request(&mut self, context: &str, event: &str) -> io::Result<Output> {
        let line = transport::exchange(
            &mut self.input,
            &mut self.output,
            &format!("{context}\t{}\n", encode(event)),
        )?;
        parse(line.trim_end())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn parse(line: &str) -> io::Result<Output> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() != 5
        || fields[0] != "W1"
        || !["0", "1"].contains(&fields[2])
        || !["0", "1"].contains(&fields[3])
    {
        return Err(io::Error::other("invalid widget response"));
    }
    let text = decode(fields[1])?;
    if text.chars().any(|ch| ch.is_control() && ch != '\n') {
        return Err(io::Error::other("widget output contains terminal controls"));
    }
    let directory = (fields[4] != "-")
        .then(|| decode(fields[4]).map(Into::into))
        .transpose()?;
    Ok(Output {
        text,
        active: fields[2] == "1",
        interactive: fields[3] == "1",
        directory,
    })
}

#[cfg(test)]
mod tests {
    use super::parse;
    #[test]
    fn validates_widget_output_and_actions() -> std::io::Result<()> {
        let output = parse("W1\t6869\t1\t1\t2f746d70")?;
        assert_eq!(output.text, "hi");
        assert!(output.active);
        assert_eq!(
            output.directory.as_deref(),
            Some(std::path::Path::new("/tmp"))
        );
        for bad in [
            "W1\t1b\t0\t0\t-",
            "W1\tff\t0\t0\t-",
            "W1\t0\t0\t0\t-",
            "W1\t\t2\t0\t-",
            "broken",
        ] {
            assert!(parse(bad).is_err());
        }
        Ok(())
    }
}
