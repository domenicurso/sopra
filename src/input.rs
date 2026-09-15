mod keys;

use std::{
    fs::OpenOptions,
    io::{self, Write},
    os::fd::AsRawFd,
    time::Duration,
};

use libc::{
    BRKINT, CS8, ECHO, ECHONL, ICANON, ICRNL, IEXTEN, IGNBRK, IGNCR, INLCR, ISIG, ISTRIP, IXON,
    OPOST, PARENB, c_int, termios, winsize,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalSize {
    pub(crate) columns: u16,
    pub(crate) rows: u16,
}

impl TerminalSize {
    pub(crate) fn new(columns: u16, rows: u16) -> Self {
        Self {
            columns: columns.max(1),
            rows: rows.max(1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CursorPosition {
    pub(crate) column: u16,
    pub(crate) row: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Key {
    Character(char),
    Enter,
    Tab,
    Backspace,
    Delete,
    WordBackspace,
    KillToEnd,
    KillToStart,
    Yank,
    Escape,
    Left,
    Right,
    WordLeft,
    WordRight,
    Up,
    Down,
    Home,
    End,
    Cancel,
    Clear,
    Eof,
}

pub(crate) struct Terminal {
    pub(super) tty: std::fs::File,
    saved_mode: termios,
    raw_mode: bool,
}

impl Terminal {
    pub(crate) fn open() -> io::Result<Self> {
        let tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
        let saved_mode = get_termios(tty.as_raw_fd())?;
        let mut raw_mode = saved_mode;
        raw_mode.c_iflag &= !(IGNBRK | BRKINT | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
        raw_mode.c_oflag &= !OPOST;
        raw_mode.c_lflag &= !(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
        raw_mode.c_cflag &= !(libc::CSIZE | PARENB);
        raw_mode.c_cflag |= CS8;
        raw_mode.c_cc[libc::VMIN] = 1;
        raw_mode.c_cc[libc::VTIME] = 0;
        set_termios(tty.as_raw_fd(), &raw_mode)?;
        Ok(Self {
            tty,
            saved_mode,
            raw_mode: true,
        })
    }

    pub(crate) fn size(&self) -> io::Result<TerminalSize> {
        let mut window = winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // ioctl writes exactly one winsize into this valid, stack-owned pointer.
        let result = unsafe { libc::ioctl(self.tty.as_raw_fd(), libc::TIOCGWINSZ, &mut window) };
        if result == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(TerminalSize::new(window.ws_col, window.ws_row))
    }

    pub(crate) fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.tty.write_all(bytes)
    }

    pub(crate) fn flush(&mut self) -> io::Result<()> {
        self.tty.flush()
    }

    pub(crate) fn poll(&self, timeout: Duration) -> io::Result<bool> {
        let fd = self.tty.as_raw_fd();
        // fd_set is a C value initialized before the macros mutate its bitset.
        let mut readfds: libc::fd_set = unsafe { std::mem::zeroed() };
        unsafe {
            libc::FD_ZERO(&mut readfds);
            libc::FD_SET(fd, &mut readfds);
        }
        let mut remaining = libc::timeval {
            tv_sec: timeout.as_secs().try_into().unwrap_or(libc::time_t::MAX),
            tv_usec: timeout.subsec_micros().try_into().unwrap_or(i32::MAX),
        };
        // macOS ptys can report POLLNVAL for /dev/tty even while select remains valid.
        let result = unsafe {
            libc::select(
                fd.saturating_add(1),
                &mut readfds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut remaining,
            )
        };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                return Ok(false);
            }
            return Err(error);
        }
        // The descriptor set is still initialized and valid for this read-only query.
        let readable = unsafe { libc::FD_ISSET(fd, &readfds) };
        Ok(result > 0 && readable)
    }

    pub(crate) fn restore(&mut self) -> io::Result<()> {
        if self.raw_mode {
            set_termios(self.tty.as_raw_fd(), &self.saved_mode)?;
            self.raw_mode = false;
        }
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn get_termios(fd: c_int) -> io::Result<termios> {
    // termios is a C POD and tcgetattr fills every field before it is read.
    let mut state = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut state) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(state)
}

fn set_termios(fd: c_int, state: &termios) -> io::Result<()> {
    // tcsetattr reads the valid termios value during this call and does not retain its pointer.
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, state) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
