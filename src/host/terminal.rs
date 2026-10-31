use portable_pty::PtySize;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::fd::AsRawFd,
};

pub(super) struct Terminal {
    file: File,
    output: File,
    saved: libc::termios,
}
impl Terminal {
    pub(super) fn open() -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
        let output = OpenOptions::new().write(true).open("/dev/tty")?;
        // tcgetattr initializes this C POD through a valid, stack-owned pointer.
        let mut saved = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(file.as_raw_fd(), &mut saved) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // This descriptor is owned by the host; nonblocking reads cannot stall PTY output.
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
        {
            return Err(io::Error::last_os_error());
        }
        let mut raw = saved;
        // cfmakeraw only mutates this initialized termios value.
        unsafe { libc::cfmakeraw(&mut raw) };
        if unsafe { libc::tcsetattr(file.as_raw_fd(), libc::TCSANOW, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut terminal = Self {
            file,
            output,
            saved,
        };
        terminal.write(b"\x1b[?1049h\x1b[?1002h\x1b[?1006h\x1b[?2004h")?;
        Ok(terminal)
    }
    pub(super) fn size(&self) -> io::Result<PtySize> {
        let mut size = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // TIOCGWINSZ writes one winsize and does not retain its pointer.
        if unsafe { libc::ioctl(self.file.as_raw_fd(), libc::TIOCGWINSZ, &mut size) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(PtySize {
            rows: size.ws_row.max(1),
            cols: size.ws_col.max(1),
            pixel_width: size.ws_xpixel,
            pixel_height: size.ws_ypixel,
        })
    }
    pub(super) fn read(&mut self) -> io::Result<Vec<u8>> {
        let mut descriptor = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // poll receives a valid array of exactly one descriptor for this call.
        let ready = unsafe { libc::poll(&mut descriptor, 1, 8) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::Interrupted {
                Ok(Vec::new())
            } else {
                Err(error)
            };
        }
        if ready == 0 {
            return Ok(Vec::new());
        }
        let mut bytes = vec![0; 65536];
        let length = match self.file.read(&mut bytes) {
            Ok(length) => length,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                return Ok(Vec::new());
            }
            Err(error) => return Err(error),
        };
        if length == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "terminal closed",
            ));
        }
        bytes.truncate(length);
        Ok(bytes)
    }
    pub(super) fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.output.write_all(bytes)?;
        self.output.flush()
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.write(b"\x1b[0m\x1b[?25h\x1b[?1002l\x1b[?1006l\x1b[?2004l\x1b[?1l\x1b[?1049l");
        // saved came from tcgetattr for this still-owned terminal descriptor.
        unsafe { libc::tcsetattr(self.file.as_raw_fd(), libc::TCSANOW, &self.saved) };
    }
}
