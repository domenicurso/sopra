use std::{
    fs::File,
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::{CompletionItem, CompletionKind, Request};

const PROVIDER_TIMEOUT: Duration = Duration::from_millis(1_500);
const RECORD_SEPARATOR: u8 = 0x1e;
const FIELD_SEPARATOR: u8 = 0x1f;

pub(super) fn capture(provider: &Path, request: &Request) -> io::Result<Vec<CompletionItem>> {
    let (mut master, slave) = open_pty()?;
    set_window_size(&master)?;
    let (mut output, output_write) = pipe()?;
    let output_fd = output_write.as_raw_fd();
    let mut child = spawn_provider(provider, request, slave, output_fd)?;
    drop(output_write);
    let mut master_reader = master.try_clone()?;
    let drain = thread::spawn(move || drain_pty(&mut master_reader));
    master.write_all(b"\x18")?;
    let records = read_provider_records(&mut child, &mut output)?;
    let _ = drain.join();
    Ok(parse_records(&records))
}

fn spawn_provider(
    provider: &Path,
    request: &Request,
    slave: File,
    output_fd: i32,
) -> io::Result<Child> {
    let slave_fd = slave.as_raw_fd();
    let mut command = Command::new("zsh");
    command
        .args(["-d", "-i"])
        .arg(provider)
        .current_dir(&request.cwd)
        .env("KEEL_PROVIDER_MODE", "1")
        .env("KEEL_PROVIDER_BUFFER", &request.provider_line)
        .env("KEEL_PROVIDER_CURSOR", request.provider_cursor.to_string())
        .env("KEEL_PROVIDER_CWD", &request.cwd);
    command.stdin(Stdio::from(slave.try_clone()?));
    command.stdout(Stdio::from(slave.try_clone()?));
    command.stderr(Stdio::from(slave));
    unsafe {
        command.pre_exec(move || {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::ioctl(slave_fd, libc::TIOCSCTTY.into(), 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::dup2(output_fd, 3) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn()
}

fn read_provider_records(child: &mut Child, output: &mut File) -> io::Result<Vec<u8>> {
    let mut records = Vec::new();
    let deadline = Instant::now() + PROVIDER_TIMEOUT;
    loop {
        if child.try_wait()?.is_some() {
            output.read_to_end(&mut records)?;
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "completion provider timed out",
            ));
        }
        thread::sleep(Duration::from_millis(5));
    }
    Ok(records)
}

fn drain_pty(master: &mut File) -> io::Result<()> {
    let mut buffer = [0_u8; 8_192];
    loop {
        match master.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.raw_os_error() == Some(libc::EIO) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn parse_records(bytes: &[u8]) -> Vec<CompletionItem> {
    bytes
        .split(|byte| *byte == RECORD_SEPARATOR)
        .filter_map(parse_record)
        .take(1_024)
        .collect()
}

fn parse_record(record: &[u8]) -> Option<CompletionItem> {
    let mut fields = record.splitn(4, |byte| *byte == FIELD_SEPARATOR);
    let label = protocol_text(fields.next()?)?;
    let kind = match fields.next()? {
        b"file" => CompletionKind::File,
        b"directory" => CompletionKind::Directory,
        b"generic" => CompletionKind::Generic,
        _ => return None,
    };
    let replacement = protocol_text(fields.next()?)?;
    let detail = protocol_text(fields.next().unwrap_or_default())?;
    (!label.is_empty() && !replacement.is_empty()).then_some(CompletionItem::new(
        label,
        detail,
        replacement,
        kind,
    ))
}

fn protocol_text(bytes: &[u8]) -> Option<String> {
    if bytes
        .iter()
        .any(|byte| matches!(*byte, 0x00 | 0x1d | 0x1e | 0x1f | b'\r' | b'\n'))
    {
        return None;
    }
    std::str::from_utf8(bytes).ok().map(str::to_string)
}

fn open_pty() -> io::Result<(File, File)> {
    let mut master = -1;
    let mut slave = -1;
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok((unsafe { File::from_raw_fd(master) }, unsafe {
        File::from_raw_fd(slave)
    }))
}

fn pipe() -> io::Result<(File, File)> {
    let mut descriptors = [-1; 2];
    if unsafe { libc::pipe(descriptors.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok((unsafe { File::from_raw_fd(descriptors[0]) }, unsafe {
        File::from_raw_fd(descriptors[1])
    }))
}

fn set_window_size(tty: &File) -> io::Result<()> {
    let size = libc::winsize {
        ws_row: 40,
        ws_col: 120,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    if unsafe { libc::ioctl(tty.as_raw_fd(), libc::TIOCSWINSZ, &size) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CompletionKind, parse_records};

    #[test]
    fn provider_records_keep_kind_replacement_and_detail() {
        let records = parse_records(b"git\x1fgeneric\x1fgit\x1fbranch\x1e");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].kind, CompletionKind::Generic);
        assert_eq!(records[0].replacement, "git");
        assert_eq!(records[0].detail, "branch");
    }

    #[test]
    fn provider_records_reject_terminal_control_bytes() {
        assert!(parse_records(b"bad\n\x1fgeneric\x1fbad\x1f\x1e").is_empty());
    }
}
