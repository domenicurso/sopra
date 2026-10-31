use super::{browser::Browser, model::Model};
use super::{
    cells::{copy_screen, highlight, text},
    layout::Layout,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

pub(super) fn frame(model: &Model, browser: Option<&Browser>) -> Buffer {
    let mut buffer = Buffer::empty(Rect::new(0, 0, model.size.cols, model.size.rows));
    let Some(browser) = browser else {
        copy_screen(&mut buffer, model.terminal.screen(), 0..model.size.rows, 0);
        return buffer;
    };
    let layout = Layout::new(model);
    let title = title(browser);
    text(
        &mut buffer,
        (0, 0),
        &title,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    history_rows(&mut buffer, browser, layout.height);
    let footer = if model.size.cols >= 75 {
        "[copy] [run] [fold] [open]  Tab block · / search · n next · Esc live"
    } else {
        "[copy] [run] [fold] [open]  Esc live"
    };
    text(
        &mut buffer,
        (0, layout.prompt_top.saturating_sub(1)),
        footer,
        Style::default().add_modifier(Modifier::DIM),
    );
    copy_screen(
        &mut buffer,
        model.terminal.screen(),
        layout.prompt_source..layout.prompt_source + layout.pinned,
        layout.prompt_top,
    );
    buffer
}

fn history_rows(buffer: &mut Buffer, browser: &Browser, height: usize) {
    for (line, row) in browser
        .rows
        .iter()
        .skip(browser.top)
        .take(height)
        .enumerate()
    {
        let style = if row.source.is_none() {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let style = if browser.selected == Some(row.block) && row.source.is_none() {
            style.bg(Color::DarkGray)
        } else {
            style
        };
        let query = !browser.query.is_empty()
            && row
                .text
                .to_lowercase()
                .contains(&browser.query.to_lowercase());
        let style = if query {
            style.add_modifier(Modifier::UNDERLINED)
        } else {
            style
        };
        let column = if row.source.is_some() { 2 } else { 0 };
        text(buffer, (column, line as u16 + 1), &row.text, style);
        if let Some(range) = browser.selection.range(row) {
            highlight(buffer, line as u16 + 1, row, range);
        }
    }
}

fn title(browser: &Browser) -> String {
    if browser.searching {
        format!("Search: {}", browser.query)
    } else {
        format!(
            "Session history  {}  {}",
            browser.notice,
            if browser.query.is_empty() {
                String::new()
            } else {
                format!("/{}", browser.query)
            }
        )
    }
}
