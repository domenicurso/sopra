use std::{
    io::{self, IsTerminal, Write},
    os::fd::AsRawFd,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(super) struct Output {
    pub(super) width: usize,
    pub(super) color: bool,
    pub(super) terminal: bool,
}
impl Output {
    pub(super) fn detect() -> Self {
        let stdout = io::stdout();
        let terminal = stdout.is_terminal();
        let mut window = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // ioctl writes one winsize into this valid stack-owned value; it does not change terminal modes.
        let measured = terminal
            && unsafe { libc::ioctl(stdout.as_raw_fd(), libc::TIOCGWINSZ, &mut window) } == 0;
        let width = if measured && window.ws_col > 0 {
            usize::from(window.ws_col)
        } else {
            80
        };
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        Self {
            width,
            terminal,
            color: terminal && !no_color && std::env::var("TERM").as_deref() != Ok("dumb"),
        }
    }
    pub(super) fn style(&self, text: &str, code: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.into()
        }
    }
}

pub(super) fn print(text: &str) -> io::Result<()> {
    io::stdout().lock().write_all(text.as_bytes())
}

pub(super) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for word in text.split_whitespace() {
        if !line.is_empty() && used + word.width() + 1 > width {
            lines.push(std::mem::take(&mut line));
            used = 0;
        }
        if !line.is_empty() {
            line.push(' ');
            used += 1;
        }
        for character in word.chars() {
            let cells = character.width().unwrap_or(0);
            if used + cells > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                used = 0;
            }
            line.push(character);
            used += cells;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrapping_counts_display_cells_and_preserves_long_tokens() {
        let lines = wrap("hello 世界 abcdefghijkl", 6);
        assert!(lines.iter().all(|line| line.width() <= 6));
        assert_eq!(lines.concat().replace(' ', ""), "hello世界abcdefghijkl");
    }
}
