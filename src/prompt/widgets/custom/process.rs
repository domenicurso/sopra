use super::transport;
use sopra::{
    Context, Event,
    protocol::{self, Response},
};
use std::{
    io::{self, BufReader},
    os::fd::AsRawFd,
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

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
        let process = Self {
            child,
            input,
            output: BufReader::new(output),
        };
        transport::nonblocking(process.input.as_raw_fd())?;
        transport::nonblocking(process.output.get_ref().as_raw_fd())?;
        Ok(process)
    }

    pub(super) fn request(
        &mut self,
        context: &Context,
        event: Option<Event>,
    ) -> io::Result<Response> {
        let line = transport::exchange(
            &mut self.input,
            &mut self.output,
            &format!("{}\n", protocol::request(context, event)),
        )?;
        protocol::read_response(&line)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
