use super::{Command, Health, SOCKET_ENV, read_response};
use std::{
    io::{self, BufReader, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) struct Client {
    socket: PathBuf,
}
pub(crate) fn connected() -> bool {
    std::env::var_os(SOCKET_ENV).is_some_and(|value| !value.is_empty())
}
impl Client {
    pub(crate) fn current() -> io::Result<Self> {
        let socket = std::env::var_os(SOCKET_ENV)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                io::Error::other("no Sopra session; run this command in a shell with Sopra enabled")
            })?;
        Ok(Self {
            socket: socket.into(),
        })
    }
    pub(crate) fn request(&self, command: Command) -> io::Result<String> {
        let mut stream = UnixStream::connect(&self.socket).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot connect to Sopra worker: {error}"),
            )
        })?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
        writeln!(stream, "C1\t{}", command.name())?;
        read_response(&mut BufReader::new(stream))
    }
    pub(crate) fn restart(&self) -> io::Result<()> {
        let before = Health::parse(&self.request(Command::Ping)?)?;
        self.request(Command::Restart)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self
                .request(Command::Ping)
                .and_then(|body| Health::parse(&body))
                .is_ok_and(|after| after.generation != before.generation)
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Err(io::Error::other(
            "worker restart did not become ready within 5 seconds",
        ))
    }
}
