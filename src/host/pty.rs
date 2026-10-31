use super::bootstrap::Bootstrap;
use super::transport::{self, Writer};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize};
use std::{io, sync::mpsc::Receiver};

pub(super) struct Shell {
    pub(super) master: Box<dyn MasterPty + Send>,
    pub(super) writer: Writer,
    pub(super) output: Receiver<io::Result<Vec<u8>>>,
    pub(super) child: Box<dyn Child + Send + Sync>,
    _bootstrap: Bootstrap,
}
impl Shell {
    pub(super) fn spawn(size: PtySize) -> io::Result<Self> {
        let bootstrap = Bootstrap::create()?;
        let pair = portable_pty::native_pty_system()
            .openpty(size)
            .map_err(io::Error::other)?;
        let mut command = CommandBuilder::new("zsh");
        command.arg("-di");
        command.cwd(std::env::current_dir()?);
        command.env("ZDOTDIR", bootstrap.directory.path());
        command.env("SOPRA_BIN", std::env::current_exe()?);
        command.env("_SOPRA_HOSTED", "1");
        command.env_remove("_SOPRA_SERVER_SOCKET");
        command.env_remove("_SOPRA_LOADED");
        // The virtual terminal supports xterm's core protocols, not host-specific graphics extensions.
        command.env("TERM", "xterm-256color");
        command.env_remove("TERM_PROGRAM");
        let writer = pair.master.take_writer().map_err(io::Error::other)?;
        let reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(io::Error::other)?;
        drop(pair.slave);
        let output = transport::reader(reader);
        let writer = Writer::new(writer);
        Ok(Self {
            master: pair.master,
            writer,
            output,
            child,
            _bootstrap: bootstrap,
        })
    }
    pub(super) fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.send(bytes)
    }
}
impl Drop for Shell {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        if let Some(group) = self.master.process_group_leader() {
            // The PTY owns this foreground process group; hang it up before closing its master.
            unsafe { libc::kill(-group, libc::SIGHUP) };
        }
        if let Some(pid) = self.child.process_id() {
            // Let Zsh hang up its jobs and close the worker socket before forced termination.
            unsafe { libc::kill(pid as i32, libc::SIGHUP) };
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
        while self.child.try_wait().ok().flatten().is_none() && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
