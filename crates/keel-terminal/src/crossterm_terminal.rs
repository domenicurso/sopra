use std::{
    collections::VecDeque,
    io::{self, Read, Stderr, Stdout, Write, stderr, stdin, stdout},
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{Hide, MoveDown, MoveToColumn, MoveUp, RestorePosition, SavePosition, Show},
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use keel_core::{CanvasBuffer, CanvasCell, CanvasRow, CellStyle, CursorStyle, InputEvent, Key, TerminalPoint, TerminalSize, VisualCursor};
use keel_render::CanvasPatch;

use crate::TerminalIo;

const BRACKETED_PASTE_START: &[u8] = b"\x1b[200~";
const BRACKETED_PASTE_END: &[u8] = b"\x1b[201~";
const OSC_FOREGROUND_QUERY: &[u8] = b"\x1b]10;?\x07";
const OSC_BACKGROUND_QUERY: &[u8] = b"\x1b]11;?\x07";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RgbColor {
    r: u8,
    g: u8,
    b: u8,
}

impl RgbColor {
    fn blend(self, target: Self, alpha: u8) -> Self {
        let alpha = alpha as u16;
        let inv_alpha = 255u16.saturating_sub(alpha);

        Self {
            r: (((self.r as u16 * inv_alpha) + (target.r as u16 * alpha)) / 255) as u8,
            g: (((self.g as u16 * inv_alpha) + (target.g as u16 * alpha)) / 255) as u8,
            b: (((self.b as u16 * inv_alpha) + (target.b as u16 * alpha)) / 255) as u8,
        }
    }
}

impl Default for RgbColor {
    fn default() -> Self {
        Self {
            r: 28,
            g: 32,
            b: 43,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThemeColors {
    foreground: RgbColor,
    background: RgbColor,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            foreground: RgbColor {
                r: 230,
                g: 232,
                b: 235,
            },
            background: RgbColor::default(),
        }
    }
}

impl ThemeColors {
    fn cursor_background(self, alpha: u8) -> Color {
        let blended = self.background.blend(self.foreground, alpha);
        Color::Rgb {
            r: blended.r,
            g: blended.g,
            b: blended.b,
        }
    }

    fn cursor_foreground(self) -> Color {
        Color::Rgb {
            r: self.background.r,
            g: self.background.g,
            b: self.background.b,
        }
    }
}

#[derive(Debug)]
enum OutputTarget {
    Stdout(Stdout),
    Stderr(Stderr),
}

impl Write for OutputTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Stdout(stdout) => stdout.write(buf),
            Self::Stderr(stderr) => stderr.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(stdout) => stdout.flush(),
            Self::Stderr(stderr) => stderr.flush(),
        }
    }
}

#[derive(Debug)]
pub struct CrosstermTerminal {
    output: OutputTarget,
    raw_mode_active: bool,
    paste_enabled: bool,
    focus_change_enabled: bool,
    cursor_row: u16,
    last_size: TerminalSize,
    input_buffer: Vec<u8>,
    paste_buffer: Vec<u8>,
    in_bracketed_paste: bool,
    pending_events: VecDeque<InputEvent>,
    theme: ThemeColors,
    current_canvas: Option<CanvasBuffer>,
}

impl CrosstermTerminal {
    pub fn enter() -> io::Result<Self> {
        Self::enter_with_output(OutputTarget::Stdout(stdout()))
    }

    pub fn enter_with_stderr() -> io::Result<Self> {
        Self::enter_with_output(OutputTarget::Stderr(stderr()))
    }

    fn enter_with_output(mut output: OutputTarget) -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(
            output,
            Hide,
            crossterm::event::EnableBracketedPaste,
            crossterm::event::EnableFocusChange
        )?;
        let (theme, leftover_input) = query_terminal_theme(&mut output)?;

        Ok(Self {
            output,
            raw_mode_active: true,
            paste_enabled: true,
            focus_change_enabled: true,
            cursor_row: 0,
            last_size: current_size()?,
            input_buffer: leftover_input,
            paste_buffer: Vec::with_capacity(256),
            in_bracketed_paste: false,
            pending_events: VecDeque::with_capacity(16),
            theme,
            current_canvas: None,
        })
    }

    pub fn restore(&mut self) -> io::Result<()> {
        if self.paste_enabled || self.focus_change_enabled {
            execute!(
                self.output,
                SetAttribute(Attribute::Reset),
                Show,
                crossterm::event::DisableBracketedPaste,
                crossterm::event::DisableFocusChange
            )?;
            self.paste_enabled = false;
            self.focus_change_enabled = false;
        }

        if self.raw_mode_active {
            disable_raw_mode()?;
            self.raw_mode_active = false;
        }

        self.output.flush()?;
        Ok(())
    }

    fn anchor_render_region(&mut self) -> io::Result<()> {
        queue!(self.output, RestorePosition)?;
        Ok(())
    }

    fn move_to_row(&mut self, current_row: u16, target_row: u16) -> io::Result<()> {
        if target_row > current_row {
            queue!(self.output, MoveDown(target_row - current_row))?;
        } else if current_row > target_row {
            queue!(self.output, MoveUp(current_row - target_row))?;
        }
        Ok(())
    }

    fn render_row(&mut self, row: &CanvasRow) -> io::Result<()> {
        queue!(self.output, MoveToColumn(0), Clear(ClearType::CurrentLine))?;

        for cell in &row.cells {
            self.render_cell(cell, None)?;
        }

        Ok(())
    }

    fn render_cell(
        &mut self,
        cell: &CanvasCell,
        cursor: Option<VisualCursor>,
    ) -> io::Result<()> {
        let symbol = if cell.symbol.is_empty() {
            " "
        } else {
            cell.symbol.as_str()
        };

        if let Some(cursor) = cursor {
            match cursor.style {
                CursorStyle::Block => queue!(
                    self.output,
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(self.theme.cursor_foreground()),
                    SetBackgroundColor(self.theme.cursor_background(cursor.alpha)),
                    Print(symbol),
                    SetForegroundColor(Color::Reset),
                    SetBackgroundColor(Color::Reset)
                )?,
                CursorStyle::Beam => queue!(
                    self.output,
                    SetAttribute(Attribute::Underlined),
                    Print(symbol),
                    SetAttribute(Attribute::Reset)
                )?,
            }

            return Ok(());
        }

        match cell.style {
            CellStyle::Prompt => queue!(
                self.output,
                SetAttribute(Attribute::Bold),
                Print(symbol),
                SetAttribute(Attribute::Reset)
            )?,
            CellStyle::Muted => queue!(
                self.output,
                SetAttribute(Attribute::Dim),
                Print(symbol),
                SetAttribute(Attribute::Reset)
            )?,
            CellStyle::Accent => queue!(
                self.output,
                SetAttribute(Attribute::Underlined),
                Print(symbol),
                SetAttribute(Attribute::Reset)
            )?,
            CellStyle::StatusOk | CellStyle::StatusError => queue!(
                self.output,
                SetAttribute(Attribute::Bold),
                Print(symbol),
                SetAttribute(Attribute::Reset)
            )?,
            CellStyle::Plain => {
                queue!(self.output, Print(symbol))?
            }
        }

        Ok(())
    }

    fn move_to_canvas_point(&mut self, point: TerminalPoint) -> io::Result<()> {
        queue!(self.output, RestorePosition)?;
        self.move_to_row(0, point.row)?;
        queue!(self.output, MoveToColumn(point.column))?;
        Ok(())
    }

    fn draw_cursor_overlay(&mut self, cursor: VisualCursor) -> io::Result<()> {
        let Some(canvas) = self.current_canvas.as_ref() else {
            return Ok(());
        };
        let row = canvas.rows.get(cursor.row as usize);
        let symbol = row
            .and_then(|row| row.cells.get(cursor.column as usize))
            .map(|cell| {
                if cell.symbol.is_empty() {
                    " ".to_string()
                } else {
                    cell.symbol.clone()
                }
            })
            .unwrap_or_else(|| " ".to_string());

        self.move_to_canvas_point(TerminalPoint {
            row: cursor.row,
            column: cursor.column,
        })?;
        self.render_cell(
            &CanvasCell {
                symbol,
                style: CellStyle::Plain,
            },
            Some(cursor),
        )?;
        Ok(())
    }

    fn poll_stdin(&mut self, timeout: Duration) -> io::Result<bool> {
        let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        let mut fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };

        loop {
            let result = unsafe { libc::poll(&mut fd, 1, timeout_ms) };
            if result >= 0 {
                return Ok(result > 0 && fd.revents & libc::POLLIN != 0);
            }

            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                let size = current_size()?;
                if size != self.last_size {
                    self.last_size = size;
                    self.pending_events.push_back(InputEvent::Resize(size));
                    return Ok(true);
                }
                continue;
            }

            return Err(error);
        }
    }

    fn read_stdin_chunk(&mut self) -> io::Result<usize> {
        let mut buffer = [0u8; 1024];
        let read = stdin().read(&mut buffer)?;
        self.input_buffer.extend_from_slice(&buffer[..read]);
        Ok(read)
    }

    fn drain_pending_events(&mut self) {
        loop {
            if self.in_bracketed_paste {
                if let Some(index) = find_subsequence(&self.input_buffer, BRACKETED_PASTE_END) {
                    self.paste_buffer.extend_from_slice(&self.input_buffer[..index]);
                    self.input_buffer.drain(..index + BRACKETED_PASTE_END.len());
                    let text = sanitize_paste(String::from_utf8_lossy(&self.paste_buffer).into());
                    self.paste_buffer.clear();
                    self.in_bracketed_paste = false;
                    self.pending_events.push_back(InputEvent::Paste(text));
                    continue;
                }

                self.paste_buffer.append(&mut self.input_buffer);
                break;
            }

            if self.input_buffer.is_empty() {
                break;
            }

            match self.try_parse_next_event() {
                ParseResult::Event(event, consumed) => {
                    self.input_buffer.drain(..consumed);
                    self.pending_events.push_back(event);
                }
                ParseResult::StartPaste(consumed) => {
                    self.input_buffer.drain(..consumed);
                    self.in_bracketed_paste = true;
                }
                ParseResult::Skip(consumed) => {
                    self.input_buffer.drain(..consumed);
                }
                ParseResult::Incomplete => break,
            }
        }
    }

    fn try_parse_next_event(&self) -> ParseResult {
        let bytes = &self.input_buffer;
        let first = bytes[0];

        match first {
            b'\r' | b'\n' => ParseResult::Event(InputEvent::Key(Key::Enter), 1),
            b'\t' => ParseResult::Event(InputEvent::Key(Key::Tab), 1),
            0x03 => ParseResult::Event(InputEvent::Key(Key::CtrlC), 1),
            0x08 | 0x7f => ParseResult::Event(InputEvent::Key(Key::Backspace), 1),
            0x1b => parse_escape_sequence(bytes),
            byte if byte < 0x20 => ParseResult::Skip(1),
            _ => parse_utf8_char(bytes),
        }
    }
}

impl Drop for CrosstermTerminal {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

impl TerminalIo for CrosstermTerminal {
    fn size(&self) -> io::Result<TerminalSize> {
        current_size()
    }

    fn enter_frontend(&mut self) -> io::Result<()> {
        if !self.raw_mode_active {
            enable_raw_mode()?;
            self.raw_mode_active = true;
        }

        queue!(self.output, SavePosition, Hide)?;
        self.output.flush()?;
        Ok(())
    }

    fn read_event(&mut self, timeout: Duration) -> io::Result<Option<InputEvent>> {
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(Some(event));
        }

        let stdin_ready = self.poll_stdin(timeout)?;
        if !stdin_ready {
            let size = current_size()?;
            if size != self.last_size {
                self.last_size = size;
                return Ok(Some(InputEvent::Resize(size)));
            }

            return Ok(None);
        }

        let read = self.read_stdin_chunk()?;
        if read == 0 {
            return Ok(None);
        }

        self.drain_pending_events();
        Ok(self.pending_events.pop_front())
    }

    fn draw(&mut self, patch: &CanvasPatch) -> io::Result<()> {
        self.anchor_render_region()?;

        if patch.full_redraw {
            queue!(self.output, Clear(ClearType::FromCursorDown))?;
        }

        let mut next_canvas = self
            .current_canvas
            .clone()
            .unwrap_or_else(|| CanvasBuffer::blank(patch.next_size));
        if patch.full_redraw || next_canvas.size != patch.next_size {
            next_canvas = CanvasBuffer::blank(patch.next_size);
        }
        next_canvas.size = patch.next_size;
        next_canvas.rows.resize_with(
            patch.next_size.rows as usize,
            || CanvasRow::blank(patch.next_size.columns),
        );
        for row in &mut next_canvas.rows {
            row.cells
                .resize(patch.next_size.columns as usize, CanvasCell::default());
        }
        for row in &patch.rows {
            if let Some(slot) = next_canvas.rows.get_mut(row.row as usize) {
                *slot = row.cells.clone();
            }
        }
        next_canvas.cursor = patch.cursor;
        self.current_canvas = Some(next_canvas);

        let mut current_row = 0u16;
        for row in &patch.rows {
            self.move_to_row(current_row, row.row)?;
            self.render_row(&row.cells)?;
            current_row = row.row;
        }

        if let Some(cursor) = patch.cursor {
            self.draw_cursor_overlay(cursor)?;
            self.cursor_row = cursor.row;
        } else {
            queue!(self.output, RestorePosition, MoveToColumn(0), Hide)?;
            self.cursor_row = 0;
        }

        self.output.flush()?;
        Ok(())
    }

    fn finalize_submission(&mut self, canvas: &CanvasBuffer, patch: &CanvasPatch) -> io::Result<()> {
        self.draw(patch)?;
        let final_point = canvas_tail_position(canvas);
        self.move_to_canvas_point(final_point)?;
        queue!(self.output, Hide)?;
        self.cursor_row = final_point.row;
        self.output.flush()?;
        Ok(())
    }

    fn suspend_for_command(&mut self) -> io::Result<()> {
        self.current_canvas = None;
        self.restore()
    }

    fn resume_frontend(&mut self, canvas: &CanvasBuffer) -> io::Result<()> {
        self.cursor_row = 0;
        self.current_canvas = Some(canvas.clone());
        self.draw(&CanvasPatch::between(None, canvas))
    }

    fn clear_frontend(&mut self, _canvas: &CanvasBuffer) -> io::Result<()> {
        self.anchor_render_region()?;
        queue!(self.output, Clear(ClearType::FromCursorDown))?;
        queue!(self.output, MoveToColumn(0), Hide)?;
        self.cursor_row = 0;
        self.current_canvas = None;
        self.output.flush()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParseResult {
    Event(InputEvent, usize),
    StartPaste(usize),
    Skip(usize),
    Incomplete,
}

fn current_size() -> io::Result<TerminalSize> {
    let (columns, rows) = terminal::size()?;
    Ok(TerminalSize { columns, rows })
}

fn poll_stdin_fd(timeout: Duration) -> io::Result<bool> {
    let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;
    let mut fd = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN,
        revents: 0,
    };

    loop {
        let result = unsafe { libc::poll(&mut fd, 1, timeout_ms) };
        if result >= 0 {
            return Ok(result > 0 && fd.revents & libc::POLLIN != 0);
        }

        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            continue;
        }

        return Err(error);
    }
}

fn read_stdin_bytes() -> io::Result<Vec<u8>> {
    let mut buffer = [0u8; 1024];
    let read = stdin().read(&mut buffer)?;
    Ok(buffer[..read].to_vec())
}

fn query_terminal_theme(output: &mut OutputTarget) -> io::Result<(ThemeColors, Vec<u8>)> {
    output.write_all(OSC_FOREGROUND_QUERY)?;
    output.write_all(OSC_BACKGROUND_QUERY)?;
    output.flush()?;

    let mut bytes = Vec::new();
    let deadline = Instant::now() + Duration::from_millis(20);

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if !poll_stdin_fd(remaining.min(Duration::from_millis(30)))? {
            continue;
        }

        let chunk = read_stdin_bytes()?;
        if chunk.is_empty() {
            break;
        }
        bytes.extend_from_slice(&chunk);

        let (foreground, background, _) = parse_terminal_theme_bytes(&bytes);
        if foreground.is_some() && background.is_some() {
            break;
        }
    }

    let (foreground, background, leftover) = parse_terminal_theme_bytes(&bytes);
    let mut theme = ThemeColors::default();
    if let Some(foreground) = foreground {
        theme.foreground = foreground;
    }
    if let Some(background) = background {
        theme.background = background;
    }

    Ok((theme, leftover))
}

fn parse_terminal_theme_bytes(bytes: &[u8]) -> (Option<RgbColor>, Option<RgbColor>, Vec<u8>) {
    let mut foreground = None;
    let mut background = None;
    let mut leftover = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] == 0x1b && index + 1 < bytes.len() && bytes[index + 1] == b']' {
            let start = index + 2;
            let Some(separator_offset) = bytes[start..].iter().position(|byte| *byte == b';') else {
                leftover.extend_from_slice(&bytes[index..]);
                break;
            };
            let separator = start + separator_offset;
            let code = &bytes[start..separator];

            let payload_start = separator + 1;
            let Some((payload_end, terminator_len)) = find_osc_terminator(bytes, payload_start) else {
                leftover.extend_from_slice(&bytes[index..]);
                break;
            };

            if let Ok(payload) = std::str::from_utf8(&bytes[payload_start..payload_end]) {
                match code {
                    b"10" => foreground = parse_osc_rgb(payload).or(foreground),
                    b"11" => background = parse_osc_rgb(payload).or(background),
                    _ => {}
                }
            }

            index = payload_end + terminator_len;
            continue;
        }

        leftover.push(bytes[index]);
        index += 1;
    }

    (foreground, background, leftover)
}

fn find_osc_terminator(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            0x07 => return Some((index, 1)),
            0x1b if index + 1 < bytes.len() && bytes[index + 1] == b'\\' => {
                return Some((index, 2));
            }
            _ => index += 1,
        }
    }

    None
}

fn parse_osc_rgb(payload: &str) -> Option<RgbColor> {
    let payload = payload.strip_prefix("rgb:")?;
    let mut components = payload.split('/');
    let r = parse_hex_component(components.next()?)?;
    let g = parse_hex_component(components.next()?)?;
    let b = parse_hex_component(components.next()?)?;
    Some(RgbColor { r, g, b })
}

fn parse_hex_component(component: &str) -> Option<u8> {
    let value = u16::from_str_radix(component, 16).ok()?;
    match component.len() {
        1 => Some((value as u8) * 17),
        2 => Some(value as u8),
        3 => Some(((value as u32 * 255) / 0x0fff) as u8),
        4 => Some((value / 257) as u8),
        _ => None,
    }
}

fn canvas_tail_position(canvas: &CanvasBuffer) -> TerminalPoint {
    let content_height = canvas.content_height();
    if content_height == 0 {
        return TerminalPoint { row: 0, column: 0 };
    }

    let row_index = content_height.saturating_sub(1) as usize;
    let row = &canvas.rows[row_index];
    let column = row
        .cells
        .iter()
        .rposition(|cell| cell.symbol != " ")
        .map(|index| index as u16 + 1)
        .unwrap_or(0);

    TerminalPoint {
        row: content_height.saturating_sub(1),
        column,
    }
}

fn parse_escape_sequence(bytes: &[u8]) -> ParseResult {
    if bytes.starts_with(BRACKETED_PASTE_START) {
        return ParseResult::StartPaste(BRACKETED_PASTE_START.len());
    }

    if bytes.starts_with(BRACKETED_PASTE_END) {
        return ParseResult::Skip(BRACKETED_PASTE_END.len());
    }

    if bytes.len() == 1 {
        return ParseResult::Incomplete;
    }

    match bytes[1] {
        b'[' => parse_csi_sequence(bytes),
        b'O' => parse_ss3_sequence(bytes),
        _ => ParseResult::Event(InputEvent::Key(Key::Esc), 1),
    }
}

fn parse_csi_sequence(bytes: &[u8]) -> ParseResult {
    if bytes.len() < 3 {
        return ParseResult::Incomplete;
    }

    match bytes[2] {
        b'A' => ParseResult::Event(InputEvent::Key(Key::Up), 3),
        b'B' => ParseResult::Event(InputEvent::Key(Key::Down), 3),
        b'C' => ParseResult::Event(InputEvent::Key(Key::Right), 3),
        b'D' => ParseResult::Event(InputEvent::Key(Key::Left), 3),
        b'H' => ParseResult::Event(InputEvent::Key(Key::Home), 3),
        b'F' => ParseResult::Event(InputEvent::Key(Key::End), 3),
        b'I' => ParseResult::Event(InputEvent::FocusGained, 3),
        b'O' => ParseResult::Event(InputEvent::FocusLost, 3),
        b'1' | b'3' | b'4' | b'7' | b'8' => parse_tilde_sequence(bytes),
        _ => ParseResult::Skip(3),
    }
}

fn parse_ss3_sequence(bytes: &[u8]) -> ParseResult {
    if bytes.len() < 3 {
        return ParseResult::Incomplete;
    }

    match bytes[2] {
        b'H' => ParseResult::Event(InputEvent::Key(Key::Home), 3),
        b'F' => ParseResult::Event(InputEvent::Key(Key::End), 3),
        _ => ParseResult::Skip(3),
    }
}

fn parse_tilde_sequence(bytes: &[u8]) -> ParseResult {
    for (index, byte) in bytes.iter().enumerate().skip(2) {
        if *byte == b'~' {
            let sequence = &bytes[2..index];
            return match sequence {
                b"1" | b"7" => ParseResult::Event(InputEvent::Key(Key::Home), index + 1),
                b"3" => ParseResult::Event(InputEvent::Key(Key::Delete), index + 1),
                b"4" | b"8" => ParseResult::Event(InputEvent::Key(Key::End), index + 1),
                b"200" => ParseResult::StartPaste(index + 1),
                b"201" => ParseResult::Skip(index + 1),
                _ => ParseResult::Skip(index + 1),
            };
        }
    }

    ParseResult::Incomplete
}

fn parse_utf8_char(bytes: &[u8]) -> ParseResult {
    let max_len = bytes.len().min(4);
    let mut needs_more = false;

    for len in 1..=max_len {
        match std::str::from_utf8(&bytes[..len]) {
            Ok(text) => {
                if let Some(ch) = text.chars().next() {
                    return ParseResult::Event(InputEvent::Key(Key::Char(ch)), len);
                }
            }
            Err(error) if error.error_len().is_none() => needs_more = true,
            Err(_) => continue,
        }
    }

    if needs_more {
        ParseResult::Incomplete
    } else {
        ParseResult::Skip(1)
    }
}

fn sanitize_paste(mut text: String) -> String {
    text = text.replace("\r\n", "\n");
    text = text.replace('\r', "\n");
    text
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::{
        BRACKETED_PASTE_END, BRACKETED_PASTE_START, ParseResult, RgbColor, canvas_tail_position,
        parse_escape_sequence, parse_osc_rgb, parse_terminal_theme_bytes, parse_utf8_char,
    };
    use keel_core::{CanvasBuffer, CursorStyle, InputEvent, Key, TerminalSize, VisualCursor};

    #[test]
    fn parses_arrow_keys_from_csi_sequences() {
        assert_eq!(
            parse_escape_sequence(b"\x1b[A"),
            ParseResult::Event(InputEvent::Key(Key::Up), 3)
        );
        assert_eq!(
            parse_escape_sequence(b"\x1b[B"),
            ParseResult::Event(InputEvent::Key(Key::Down), 3)
        );
        assert_eq!(
            parse_escape_sequence(b"\x1b[C"),
            ParseResult::Event(InputEvent::Key(Key::Right), 3)
        );
        assert_eq!(
            parse_escape_sequence(b"\x1b[D"),
            ParseResult::Event(InputEvent::Key(Key::Left), 3)
        );
    }

    #[test]
    fn detects_bracketed_paste_boundaries() {
        assert_eq!(
            parse_escape_sequence(BRACKETED_PASTE_START),
            ParseResult::StartPaste(BRACKETED_PASTE_START.len())
        );
        assert_eq!(
            parse_escape_sequence(BRACKETED_PASTE_END),
            ParseResult::Skip(BRACKETED_PASTE_END.len())
        );
    }

    #[test]
    fn parses_utf8_scalar_values() {
        assert_eq!(
            parse_utf8_char("é".as_bytes()),
            ParseResult::Event(InputEvent::Key(Key::Char('é')), "é".len())
        );
        assert_eq!(
            parse_utf8_char("👍".as_bytes()),
            ParseResult::Event(InputEvent::Key(Key::Char('👍')), "👍".len())
        );
    }

    #[test]
    fn parses_terminal_theme_osc_responses() {
        let bytes = b"\x1b]10;rgb:eeee/eeee/eeee\x07\x1b]11;rgb:1c1c/2020/2b2b\x1b\\";
        let (foreground, background, leftover) = parse_terminal_theme_bytes(bytes);

        assert_eq!(
            foreground,
            Some(RgbColor {
                r: 238,
                g: 238,
                b: 238,
            })
        );
        assert_eq!(
            background,
            Some(RgbColor {
                r: 28,
                g: 32,
                b: 43,
            })
        );
        assert!(leftover.is_empty());
    }

    #[test]
    fn parses_rgb_payloads_with_variable_precision() {
        assert_eq!(
            parse_osc_rgb("rgb:f/8/0"),
            Some(RgbColor {
                r: 255,
                g: 136,
                b: 0,
            })
        );
        assert_eq!(
            parse_osc_rgb("rgb:ffff/0000/8000"),
            Some(RgbColor {
                r: 255,
                g: 0,
                b: 127,
            })
        );
    }

    #[test]
    fn computes_canvas_tail_from_last_non_blank_cell() {
        let mut canvas = CanvasBuffer::blank(TerminalSize {
            columns: 8,
            rows: 4,
        });
        canvas.rows[0].cells[0].symbol = ">".to_string();
        canvas.rows[1].cells[0].symbol = "h".to_string();
        canvas.rows[1].cells[1].symbol = "i".to_string();
        canvas.cursor = Some(VisualCursor {
            row: 1,
            column: 2,
            style: CursorStyle::Block,
            visible: true,
            alpha: 255,
        });

        let point = canvas_tail_position(&canvas);
        assert_eq!(point.row, 1);
        assert_eq!(point.column, 2);
    }
}
