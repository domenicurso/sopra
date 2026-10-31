use super::{
    actions::{self, Action},
    blocks::Blocks,
    browser::Browser,
    input::Mouse,
    layout::Layout,
};

pub(super) fn route(
    mouse: Mouse,
    browser: &mut Browser,
    blocks: &mut Blocks,
    layout: &Layout,
) -> Action {
    if mouse.button & 64 != 0 {
        browser.scroll(if mouse.button & 1 == 0 { -3 } else { 3 }, layout.height);
        return Action::None;
    }
    if mouse.release || mouse.button & 3 != 0 {
        return Action::None;
    }
    if mouse.row == layout.prompt_top.saturating_sub(1) && mouse.button & 32 == 0 {
        return footer(mouse.column, browser, blocks);
    }
    let Some(index) = mouse
        .row
        .checked_sub(1)
        .map(|row| browser.top + usize::from(row))
    else {
        return Action::None;
    };
    let Some(row) = browser.rows.get(index) else {
        return Action::None;
    };
    browser.selected = Some(row.block);
    let Some(point) = actions::point(row, mouse.column) else {
        if mouse.button & 32 != 0 {
            return Action::None;
        }
        return header(mouse.column, row, browser, blocks);
    };
    if mouse.button & 16 != 0 {
        return actions::path(point, blocks);
    }
    if mouse.button & 32 != 0 {
        browser.selection.drag(point);
    } else {
        browser.selection.press(point, blocks);
    }
    Action::None
}

fn footer(column: u16, browser: &Browser, blocks: &mut Blocks) -> Action {
    match column {
        0..=5 => actions::copy(browser, blocks),
        7..=11 => actions::run(browser, blocks, true),
        13..=18 => {
            if let Some(id) = browser.selected {
                blocks.toggle(id);
            }
            Action::Refresh
        }
        20..=25 => actions::open(browser, blocks),
        _ => Action::None,
    }
}
fn header(column: u16, row: &super::rows::Row, browser: &Browser, blocks: &mut Blocks) -> Action {
    if row.text.find("[copy]").is_some_and(|start| {
        let start = super::cells::width(&row.text[..start]);
        (start..start + 6).contains(&column)
    }) {
        return actions::copy(browser, blocks);
    }
    if row.text.find("[run]").is_some_and(|start| {
        let start = super::cells::width(&row.text[..start]);
        (start..start + 5).contains(&column)
    }) {
        return actions::run(browser, blocks, true);
    }
    blocks.toggle(row.block);
    Action::Refresh
}
