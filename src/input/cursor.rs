use std::{
    os::fd::RawFd,
    time::{Duration, Instant},
};

const RESPONSE_TIMEOUT: Duration = Duration::from_millis(100);
const CURSOR_QUERY: &[u8] = b"\x1b[6n";

pub(super) fn read(fd: RawFd) -> (Option<u16>, Vec<u8>) {
    // fcntl reads flags from this live terminal descriptor without retaining its pointer.
    let original_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if original_flags < 0 {
        return (None, Vec::new());
    }
    let nonblocking = original_flags | libc::O_NONBLOCK;
    // fcntl changes only this descriptor's flags and does not retain the borrowed value.
    if unsafe { libc::fcntl(fd, libc::F_SETFL, nonblocking) } < 0 {
        return (None, Vec::new());
    }

    let bytes = if write_query(fd) {
        read_response(fd)
    } else {
        Vec::new()
    };
    let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, original_flags) };
    parse_response(&bytes)
}

fn write_query(fd: RawFd) -> bool {
    let mut written = 0;
    while written < CURSOR_QUERY.len() {
        let remaining = &CURSOR_QUERY[written..];
        // The query slice remains alive for the duration of this synchronous libc call.
        let count = unsafe { libc::write(fd, remaining.as_ptr().cast(), remaining.len()) };
        if count <= 0 {
            return false;
        }
        written += count as usize;
    }
    true
}

fn read_response(fd: RawFd) -> Vec<u8> {
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
        // poll writes only the revents field of this valid, stack-owned descriptor.
        let ready = unsafe { libc::poll(&mut descriptor, 1, milliseconds) };
        if ready <= 0 {
            break;
        }
        let mut buffer = [0_u8; 256];
        // The buffer is valid for the requested length and is read only during this call.
        let count = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        if count < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
                continue;
            }
            break;
        }
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count as usize]);
        if find_response(&bytes).is_some() {
            break;
        }
    }
    bytes
}

fn parse_response(bytes: &[u8]) -> (Option<u16>, Vec<u8>) {
    let Some((start, end, row)) = find_response(bytes) else {
        return (None, bytes.to_vec());
    };
    let mut pending = Vec::with_capacity(bytes.len().saturating_sub(end - start));
    pending.extend_from_slice(&bytes[..start]);
    pending.extend_from_slice(&bytes[end..]);
    (row.checked_sub(1), pending)
}

fn find_response(bytes: &[u8]) -> Option<(usize, usize, u16)> {
    for start in bytes
        .windows(2)
        .enumerate()
        .filter_map(|(index, window)| (window == b"\x1b[").then_some(index))
    {
        let mut cursor = start + 2;
        let row_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        let row_end = cursor;
        if cursor == row_start || bytes.get(cursor) != Some(&b';') {
            continue;
        }
        cursor += 1;
        let column_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == column_start || bytes.get(cursor) != Some(&b'R') {
            continue;
        }
        let row = std::str::from_utf8(&bytes[row_start..row_end])
            .ok()?
            .parse()
            .ok()?;
        return Some((start, cursor + 1, row));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_response;

    #[test]
    fn extracts_cursor_response_and_preserves_other_input() {
        let (row, pending) = parse_response(b"x\x1b[12;34Ry");
        assert_eq!(row, Some(11));
        assert_eq!(pending, b"xy");
    }

    #[test]
    fn ignores_incomplete_or_non_cursor_escape_sequences() {
        assert_eq!(parse_response(b"\x1b[A"), (None, b"\x1b[A".to_vec()));
        assert_eq!(parse_response(b"\x1b[12;"), (None, b"\x1b[12;".to_vec()));
    }
}
