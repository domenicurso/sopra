use super::*;
use std::time::Duration;

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

pub(super) struct Loop {
    pub(super) renderer: Renderer,
    resize_epoch: usize,
    next_frame: Instant,
    resize_deadline: Instant,
}
impl Loop {
    pub(super) fn new(terminal: &Terminal) -> Self {
        Self {
            renderer: Renderer::editor(),
            resize_epoch: terminal.resize_epoch(),
            next_frame: Instant::now(),
            resize_deadline: Instant::now(),
        }
    }

    pub(super) fn step(
        &mut self,
        editor: &mut EditorState,
        terminal: &mut Terminal,
        syntax: &mut super::super::submission::Checker,
    ) -> io::Result<Option<RunResult>> {
        if let Some(result) = editor.poll_actions(syntax)? {
            return Ok(Some(result));
        }
        self.observe_resize(terminal.resize_epoch());
        let now = Instant::now();
        self.next_frame = self.next_frame.max(self.resize_deadline);
        if now >= self.next_frame {
            editor.paint_editor(now, terminal, &mut self.renderer)?;
            self.next_frame = now + FRAME_INTERVAL;
        }
        let wait = self.next_frame.saturating_duration_since(Instant::now());
        // Typeahead must not overtake a continuation or directory change, or be consumed
        // from the foreground command that will follow this edit.
        let result = if syntax.pending() || editor.widgets.pending() {
            std::thread::sleep(wait);
            None
        } else if terminal.poll(wait)? {
            let key = crate::actions::resolve(terminal.read_key()?);
            self.next_frame = Instant::now();
            if key == Action::Enter && !editor.widgets.active() {
                syntax.request(&editor.buffer)?;
                None
            } else {
                syntax.cancel();
                editor.handle_input(key, terminal, &mut self.renderer, &mut self.resize_epoch)?
            }
        } else {
            None
        };
        self.observe_resize(terminal.resize_epoch());
        Ok(result)
    }

    fn observe_resize(&mut self, latest: usize) {
        if latest != self.resize_epoch {
            self.resize_epoch = latest;
            // A cursor reply and a frame must describe the same terminal size.
            // Coalesce a burst before querying or painting its final geometry.
            self.resize_deadline = Instant::now() + FRAME_INTERVAL;
            self.next_frame = self.resize_deadline;
        }
    }
}
