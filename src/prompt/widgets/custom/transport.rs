use std::{
    io::{self, BufRead, BufReader, Write},
    os::fd::AsRawFd,
    process::{ChildStdin, ChildStdout},
    time::{Duration, Instant},
};

pub(super) fn nonblocking(fd: i32) -> io::Result<()> {
    // fcntl only changes status flags on this owned, open pipe descriptor.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(super) fn exchange(
    input: &mut ChildStdin,
    output: &mut BufReader<ChildStdout>,
    message: &str,
) -> io::Result<String> {
    let deadline = Instant::now() + Duration::from_millis(100);
    send(input, message.as_bytes(), deadline)?;
    let mut line = Vec::new();
    while line.last() != Some(&b'\n') {
        if output.buffer().is_empty() {
            wait(output.get_ref().as_raw_fd(), libc::POLLIN, deadline)?;
        }
        let bytes = match output.fill_buf() {
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
            result => result?,
        };
        if bytes.is_empty() {
            return Err(io::Error::other("widget exited"));
        }
        let length = bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |end| end + 1);
        line.extend_from_slice(&bytes[..length]);
        output.consume(length);
        if line.len() > 65536 {
            return Err(io::Error::other("widget output exceeds 64 KiB"));
        }
    }
    String::from_utf8(line).map_err(io::Error::other)
}

fn send(input: &mut ChildStdin, mut bytes: &[u8], deadline: Instant) -> io::Result<()> {
    while !bytes.is_empty() {
        wait(input.as_raw_fd(), libc::POLLOUT, deadline)?;
        match input.write(bytes) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "widget input closed",
                ));
            }
            Ok(length) => bytes = &bytes[length..],
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn wait(fd: i32, events: i16, deadline: Instant) -> io::Result<()> {
    let mut poll = libc::pollfd {
        fd,
        events,
        revents: 0,
    };
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(timeout());
        }
        // poll borrows one valid child pipe descriptor until it returns.
        let ready = unsafe { libc::poll(&mut poll, 1, remaining.as_millis().max(1) as i32) };
        if ready > 0 {
            return Ok(());
        }
        if ready == 0 {
            return Err(timeout());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

fn timeout() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "widget response exceeded 100 ms")
}
