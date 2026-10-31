use super::{Session, input::Mouse};
use std::io;

impl Session {
    pub(super) fn input_packet(&mut self, packet: &[u8]) -> io::Result<()> {
        if packet == b"\x1b[200~" {
            self.pasted = true;
        }
        if self.pasted {
            self.model.live();
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
        if !self.model.terminal.screen().alternate_screen() {
            match packet {
                b"\x1b[5~" => {
                    self.model
                        .scroll(isize::try_from(self.model.size.rows).unwrap_or(1));
                    return Ok(());
                }
                b"\x1b[6~" => {
                    self.model
                        .scroll(-isize::try_from(self.model.size.rows).unwrap_or(1));
                    return Ok(());
                }
                _ => self.model.live(),
            }
        }
        self.shell.send(packet)
    }
    fn mouse_packet(&mut self, event: Mouse) -> io::Result<()> {
        let screen = self.model.terminal.screen();
        if event.button & 64 != 0 && !screen.alternate_screen() {
            self.model
                .scroll(if event.button & 1 == 0 { 3 } else { -3 });
            return Ok(());
        }
        if screen.scrollback() == 0
            && screen.mouse_protocol_mode() != vt100::MouseProtocolMode::None
        {
            self.shell
                .send(&event.encode(screen.mouse_protocol_encoding()))?;
        }
        Ok(())
    }
}
