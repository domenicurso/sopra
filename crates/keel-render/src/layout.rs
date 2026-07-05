use keel_core::TerminalSize;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptLayoutRegions {
    pub left_capacity: usize,
    pub right_origin: usize,
}

pub fn right_prompt_regions(
    terminal_size: TerminalSize,
    right_prompt_width: usize,
) -> Option<PromptLayoutRegions> {
    let total_width = terminal_size.columns;
    let right_width = u16::try_from(right_prompt_width).ok()?;

    if total_width == 0 || right_width == 0 || right_width >= total_width {
        return None;
    }

    let regions = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_width)])
        .split(Rect::new(0, 0, total_width, 1));

    Some(PromptLayoutRegions {
        left_capacity: regions[0].width as usize,
        right_origin: regions[1].x as usize,
    })
}
