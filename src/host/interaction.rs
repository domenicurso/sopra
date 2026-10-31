use super::{
    Session,
    actions::{self, Action},
    input::Mouse,
    layout::Layout,
    mouse,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::io;

impl Session {
    pub(super) fn input_packet(&mut self, packet: &[u8]) -> io::Result<()> {
        if packet == b"\x1b[200~" {
            self.pasted = true;
        }
        if self.pasted {
            if !matches!(packet, b"\x1b[200~" | b"\x1b[201~")
                || self.model.terminal.screen().bracketed_paste()
            {
                self.shell.send(packet)?;
            }
            if packet == b"\x1b[201~" {
                self.pasted = false;
            }
            return Ok(());
        }
        if let Some(mouse) = Mouse::parse(packet) {
            return self.mouse_packet(mouse);
        }
        if packet == b"\x1bh"
            || packet == b"\x1b/"
            || (packet == b"\x1b[5~" && !self.model.terminal.screen().alternate_screen())
        {
            self.browse()?;
            if packet == b"\x1b/"
                && let Some(browser) = &mut self.browser
            {
                browser.searching = true;
                browser.query.clear();
            }
            return Ok(());
        }
        let Some(browser) = &mut self.browser else {
            return self.shell.send(packet);
        };
        let height = Layout::new(&self.model).height;
        let action = actions::keyboard(packet, browser, &mut self.model.blocks, height);
        self.apply(action)
    }
    fn mouse_packet(&mut self, mut event: Mouse) -> io::Result<()> {
        let screen = self.model.terminal.screen();
        if screen.alternate_screen() && self.browser.is_none() {
            return if screen.mouse_protocol_mode() == vt100::MouseProtocolMode::None {
                Ok(())
            } else {
                self.shell
                    .send(&event.encode(screen.mouse_protocol_encoding()))
            };
        }
        if event.button & 64 != 0 {
            self.browse()?;
        }
        if let Some(browser) = &mut self.browser {
            let layout = Layout::new(&self.model);
            if event.button & 64 != 0 || event.row < layout.prompt_top {
                let action = mouse::route(event, browser, &mut self.model.blocks, &layout);
                return self.apply(action);
            }
            event.row = event
                .row
                .saturating_sub(layout.prompt_top)
                .saturating_add(layout.prompt_source);
        }
        let screen = self.model.terminal.screen();
        if screen.mouse_protocol_mode() != vt100::MouseProtocolMode::None {
            self.shell
                .send(&event.encode(screen.mouse_protocol_encoding()))?;
        }
        Ok(())
    }
    fn apply(&mut self, action: Action) -> io::Result<()> {
        match action {
            Action::None => {}
            Action::Live => self.browser = None,
            Action::Send(bytes) => self.shell.send(&bytes)?,
            Action::Copy(text) => {
                self.terminal
                    .write(format!("\x1b]52;c;{}\x1b\\", STANDARD.encode(text)).as_bytes())?;
                if let Some(browser) = &mut self.browser {
                    browser.notice = "Copied".into();
                }
            }
            Action::Run(command, execute) => {
                if self.model.executing || self.model.secondary || self.model.prompt.is_none() {
                    if let Some(browser) = &mut self.browser {
                        browser.notice = "Return to a primary prompt before rerunning".into();
                    }
                    return Ok(());
                }
                let mode = if execute { 'r' } else { 'e' };
                self.shell.send(
                    format!(
                        "\x1b]777;sopra;buffer;{mode}{}\x07",
                        STANDARD.encode(command)
                    )
                    .as_bytes(),
                )?;
                self.browser = None;
            }
            Action::Open(path) => {
                if let Err(error) = self.openers.start(path.clone()) {
                    if let Some(browser) = &mut self.browser {
                        browser.notice = format!("Could not open {}: {error}", path.display());
                    }
                } else if let Some(browser) = &mut self.browser {
                    browser.notice = format!("Opening {}", path.display());
                }
            }
            Action::Refresh => self.refresh_browser(),
        }
        Ok(())
    }
}
