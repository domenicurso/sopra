use super::EditorState;
use crate::{actions::Action, input::Terminal};
use std::io;

impl EditorState {
    pub(super) fn mouse_mode_input(
        &mut self,
        key: Action,
        terminal: &mut Terminal,
    ) -> io::Result<bool> {
        let enabled = match key {
            Action::SearchHistory => {
                self.set_mouse_editing(false, terminal)?;
                return Ok(false);
            }
            Action::ToggleMouseEditing if self.searching_history() => return Ok(true),
            Action::ToggleMouseEditing => !self.mouse_editing,
            Action::Escape if self.mouse_editing => false,
            Action::Enter => {
                self.set_mouse_editing(false, terminal)?;
                return Ok(false);
            }
            _ => return Ok(false),
        };
        self.set_mouse_editing(enabled, terminal)?;
        Ok(true)
    }

    pub(super) fn set_mouse_editing(
        &mut self,
        enabled: bool,
        terminal: &mut Terminal,
    ) -> io::Result<()> {
        terminal.set_mouse_reporting(enabled)?;
        self.mouse_editing = enabled;
        if !enabled {
            self.text_selection.clear();
        }
        Ok(())
    }

    pub(super) fn mouse_mode_hint(&mut self) {
        if self.mouse_editing {
            self.rprompt = vec![crate::prompt::PromptSpan {
                text: "MOUSE".into(),
                style: ratatui::style::Style::new().dim(),
            }];
        }
    }
}
