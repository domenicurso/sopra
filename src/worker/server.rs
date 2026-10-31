use crate::control::{self, Command, Health};
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

pub(super) enum Operation {
    Edit(PathBuf),
    Control(Command),
}
pub(super) struct Connection {
    pub(super) operation: Operation,
    pub(super) stream: UnixStream,
    busy: Arc<AtomicBool>,
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}

pub(super) fn listen(socket: &Path, health: Health) -> io::Result<mpsc::Receiver<Connection>> {
    let listener = UnixListener::bind(socket)?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let busy = Arc::new(AtomicBool::new(false));
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            match receive(&mut stream, &health, &busy) {
                Ok(Some(operation)) => {
                    if sender
                        .send(Connection {
                            operation,
                            stream,
                            busy: busy.clone(),
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = writeln!(stream, "E1\t{error}");
                }
            }
        }
    });
    Ok(receiver)
}

fn receive(
    stream: &mut UnixStream,
    health: &Health,
    busy: &AtomicBool,
) -> io::Result<Option<Operation>> {
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    let mut line = String::new();
    BufReader::new(&mut *stream)
        .take(4097)
        .read_line(&mut line)?;
    if line.len() > 4096 || !line.ends_with('\n') {
        return Err(io::Error::other("invalid worker request"));
    }
    let control = match control::read_request(&line) {
        Ok(command) => command,
        Err(error) => {
            control::write_response(stream, Err(error))?;
            return Ok(None);
        }
    };
    if control == Some(Command::Ping) {
        control::write_response(stream, Ok(health.encode(busy.load(Ordering::Acquire))))?;
        return Ok(None);
    }
    if busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        let error = io::Error::other(
            "worker is editing; finish the current command before changing the session",
        );
        if control.is_some() {
            control::write_response(stream, Err(error))?
        } else {
            writeln!(stream, "E1\t{error}")?
        }
        return Ok(None);
    }
    Ok(Some(control.map_or_else(
        || Operation::Edit(line.trim_end_matches('\n').into()),
        Operation::Control,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_sessions_remain_inspectable_and_reject_mutations() -> io::Result<()> {
        let health = Health::new("/config.toml".into(), 2)?;
        let busy = AtomicBool::new(true);
        for command in ["refresh", "restart", "config-show", "widgets-list", "ping"] {
            let (mut client, mut server) = UnixStream::pair()?;
            writeln!(client, "C1\t{command}")?;
            assert!(receive(&mut server, &health, &busy)?.is_none());
            let response = control::read_response(&mut BufReader::new(client));
            if command == "ping" {
                assert!(Health::parse(&response?)?.busy);
            } else {
                assert!(
                    response.is_err_and(|error| error.to_string().contains("worker is editing"))
                );
            }
        }
        Ok(())
    }

    #[test]
    fn malformed_requests_do_not_claim_the_session() -> io::Result<()> {
        let health = Health::new("/config.toml".into(), 2)?;
        let busy = AtomicBool::new(false);
        let (mut client, mut server) = UnixStream::pair()?;
        writeln!(client, "C1\tunknown")?;
        assert!(receive(&mut server, &health, &busy)?.is_none());
        assert!(control::read_response(&mut BufReader::new(client)).is_err());
        assert!(!busy.load(Ordering::Acquire));
        let (mut client, mut server) = UnixStream::pair()?;
        client.write_all(&[b'x'; 4097])?;
        assert!(receive(&mut server, &health, &busy).is_err());
        assert!(!busy.load(Ordering::Acquire));
        Ok(())
    }

    #[test]
    fn dropped_connections_release_the_session() -> io::Result<()> {
        let health = Health::new("/config.toml".into(), 2)?;
        let busy = Arc::new(AtomicBool::new(false));
        let (mut client, mut stream) = UnixStream::pair()?;
        writeln!(client, "C1\trefresh")?;
        let operation = receive(&mut stream, &health, &busy)?
            .ok_or_else(|| io::Error::other("request was not admitted"))?;
        assert!(busy.load(Ordering::Acquire));
        drop(Connection {
            operation,
            stream,
            busy: busy.clone(),
        });
        assert!(!busy.load(Ordering::Acquire));
        Ok(())
    }
}
