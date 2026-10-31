use super::callbacks::Callbacks;
use portable_pty::PtySize;
use std::num::NonZeroU16;

pub(super) struct Model {
    pub(super) terminal: vt100::Parser<Callbacks>,
    pub(super) size: PtySize,
}
impl Model {
    pub(super) fn new(size: PtySize, lines: usize) -> Self {
        let (rows, cols) = dimensions(size);
        Self {
            terminal: vt100::Parser::new_with_callbacks(rows, cols, lines, Callbacks::default()),
            size,
        }
    }
    pub(super) fn process(&mut self, bytes: &[u8]) {
        self.terminal.process(bytes);
    }
    pub(super) fn resize(&mut self, size: PtySize) {
        let offset = self.terminal.screen().scrollback();
        let (rows, cols) = dimensions(size);
        self.terminal.set_size(rows, cols);
        self.terminal.screen_mut().set_scrollback(offset);
        self.size = size;
    }
    pub(super) fn scroll(&mut self, delta: isize) {
        let screen = self.terminal.screen_mut();
        if !screen.alternate_screen() {
            screen.set_scrollback(screen.scrollback().saturating_add_signed(delta));
        }
    }
    pub(super) fn live(&mut self) {
        self.terminal.screen_mut().set_scrollback(0);
    }
}
fn dimensions(size: PtySize) -> (NonZeroU16, NonZeroU16) {
    (
        NonZeroU16::new(size.rows).unwrap_or(NonZeroU16::MIN),
        NonZeroU16::new(size.cols).unwrap_or(NonZeroU16::MIN),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scrolling_shows_the_original_output_and_typing_returns_to_the_live_screen() {
        let mut model = Model::new(
            PtySize {
                rows: 4,
                cols: 40,
                ..PtySize::default()
            },
            100,
        );
        model.process(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\n$ draft");
        let live = model.terminal.screen().contents();
        model.scroll(3);
        assert!(model.terminal.screen().contents().starts_with("one\ntwo\n"));
        model.scroll(isize::MAX);
        assert_eq!(
            model.terminal.screen().scrollback(),
            model.terminal.screen().scrollback_len()
        );
        model.live();
        assert_eq!(model.terminal.screen().contents(), live);
    }
    #[test]
    fn arriving_output_keeps_the_scrolled_view_anchored() {
        let mut model = Model::new(
            PtySize {
                rows: 3,
                cols: 40,
                ..PtySize::default()
            },
            100,
        );
        model.process(b"one\r\ntwo\r\nthree\r\nfour\r\nfive");
        model.scroll(2);
        let earlier = model.terminal.screen().contents();
        model.process(b"\r\nsix\r\nseven");
        assert_eq!(model.terminal.screen().contents(), earlier);
        assert_eq!(model.terminal.screen().scrollback(), 4);
        model.live();
        assert!(model.terminal.screen().contents().ends_with("seven"));
    }
    #[test]
    fn alternate_screen_is_not_scrolled_and_history_is_bounded() {
        let mut model = Model::new(
            PtySize {
                rows: 2,
                cols: 20,
                ..PtySize::default()
            },
            3,
        );
        for _ in 0..30 {
            model.process(b"line\r\n");
        }
        assert_eq!(model.terminal.screen().scrollback_len(), 3);
        model.process(b"\x1b[?1049hprogram");
        model.scroll(3);
        assert_eq!(model.terminal.screen().scrollback(), 0);
        assert!(model.terminal.screen().contents().contains("program"));
        model.process(b"\x1b[?1049l");
        model.scroll(3);
        assert_eq!(model.terminal.screen().scrollback(), 3);
    }
}
