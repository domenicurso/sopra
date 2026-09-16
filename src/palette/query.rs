use std::{
    os::fd::RawFd,
    time::{Duration, Instant},
};

use super::parser::parse_responses;

const RESPONSE_TIMEOUT: Duration = Duration::from_millis(100);
const COLOR_QUERY: &[u8] = b"\x1b]11;?\x1b\\\x1b]12;?\x1b\\";

pub(super) fn read(fd: RawFd) -> (super::TerminalPalette, Vec<u8>) {
    // fcntl reads the flags for this live terminal descriptor without retaining its pointer.
    let original_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if original_flags < 0 {
        return (super::TerminalPalette::default(), Vec::new());
    }
    let nonblocking = original_flags | libc::O_NONBLOCK;
    // fcntl changes only this descriptor's flags and does not retain the borrowed value.
    if unsafe { libc::fcntl(fd, libc::F_SETFL, nonblocking) } < 0 {
        return (super::TerminalPalette::default(), Vec::new());
    }

    let bytes = if write_query(fd) {
        read_responses(fd)
    } else {
        Vec::new()
    };
    // Restore the shell's original blocking mode before any key can be consumed.
    let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, original_flags) };
    parse_responses(&bytes)
}

fn write_query(fd: RawFd) -> bool {
    let mut written = 0;
    while written < COLOR_QUERY.len() {
        let remaining = &COLOR_QUERY[written..];
        // The query slice remains alive for the duration of this synchronous libc call.
        let count = unsafe { libc::write(fd, remaining.as_ptr().cast(), remaining.len()) };
        if count <= 0 {
            return false;
        }
        written += count as usize;
    }
    true
}

fn read_responses(fd: RawFd) -> Vec<u8> {
    let deadline = Instant::now() + RESPONSE_TIMEOUT;
    let mut bytes = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let milliseconds = remaining.as_millis().min(i32::MAX as u128) as i32;
        let mut descriptor = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        // poll writes the revents field of this valid, stack-owned descriptor.
        let ready = unsafe { libc::poll(&mut descriptor, 1, milliseconds) };
        if ready <= 0 {
            break;
        }
        let mut buffer = [0_u8; 256];
        // The buffer is valid for the requested length and is read only during this call.
        let count = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        if count < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            break;
        }
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count as usize]);
        if has_response(&bytes, b"11") && has_response(&bytes, b"12") {
            break;
        }
    }
    bytes
}

fn has_response(bytes: &[u8], code: &[u8]) -> bool {
    let mut marker = Vec::with_capacity(code.len() + 3);
    marker.extend_from_slice(b"\x1b]");
    marker.extend_from_slice(code);
    marker.push(b';');
    bytes.windows(marker.len()).any(|window| window == marker)
}
