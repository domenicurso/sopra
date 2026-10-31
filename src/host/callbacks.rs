#[derive(Default)]
pub(super) struct Callbacks {
    pub(super) replies: Vec<u8>,
    pub(super) terminal: Vec<u8>,
}
impl vt100::Callbacks for Callbacks {
    fn unhandled_osc(&mut self, _screen: &mut vt100::Screen, params: &[&[u8]]) {
        if matches!(params.first(), Some(&b"10" | &b"11" | &b"12" | &b"4"))
            && params.contains(&b"?".as_slice())
        {
            self.terminal.extend_from_slice(b"\x1b]");
            self.terminal.extend_from_slice(&params.join(&b';'));
            self.terminal.push(7);
        }
    }
    // vt100's callback contract fixes this signature; the host never exposes it as its own API.
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        _: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        let first = params
            .first()
            .and_then(|value| value.first())
            .copied()
            .unwrap_or(0);
        let response = match (i1, c, first) {
            (None, 'n', 6) => {
                let (row, column) = screen.cursor_position();
                format!("\x1b[{};{}R", row + 1, column + 1)
            }
            (None, 'n', 5) => "\x1b[0n".into(),
            (None, 'c', _) => "\x1b[?1;2c".into(),
            (Some(b'>'), 'c', _) => "\x1b[>0;136;0c".into(),
            (Some(b'?'), 'u', _) => "\x1b[?0u".into(),
            (None, 't', 18) => {
                let (rows, columns) = screen.size();
                format!("\x1b[8;{rows};{columns}t")
            }
            (Some(b'?'), 'p', mode) => format!("\x1b[?{mode};0$y"),
            _ => String::new(),
        };
        self.replies.extend_from_slice(response.as_bytes());
    }
    fn audible_bell(&mut self, _: &mut vt100::Screen) {
        self.terminal.push(7);
    }
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.terminal.extend_from_slice(b"\x1b]2;");
        self.terminal.extend_from_slice(title);
        self.terminal.push(7);
    }
    fn copy_to_clipboard(&mut self, _: &mut vt100::Screen, ty: &[u8], data: &[u8]) {
        self.terminal.extend_from_slice(b"\x1b]52;");
        self.terminal.extend_from_slice(ty);
        self.terminal.push(b';');
        self.terminal.extend_from_slice(data);
        self.terminal.push(7);
    }
}
