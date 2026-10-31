use super::{blocks::Blocks, browser::Browser, files, rows::Section, selection::Point};
use std::path::PathBuf;

pub(super) enum Action {
    None,
    Live,
    Send(Vec<u8>),
    Copy(String),
    Run(String, bool),
    Open(PathBuf),
    Refresh,
}

pub(super) fn keyboard(
    bytes: &[u8],
    browser: &mut Browser,
    blocks: &mut Blocks,
    height: usize,
) -> Action {
    if bytes == b"\x1b" {
        if browser.searching {
            browser.searching = false;
            return Action::None;
        }
        return Action::Live;
    }
    if browser.searching {
        search_input(bytes, browser, blocks);
        return Action::None;
    }
    match bytes {
        b"\x1b[A" | b"\x1bOA" => browser.scroll(-1, height),
        b"\x1b[B" | b"\x1bOB" => browser.scroll(1, height),
        b"\x1b[5~" => browser.scroll(-(height as isize), height),
        b"\x1b[6~" => browser.scroll(height as isize, height),
        b"\t" => browser.next_block(false),
        b"\x1b[Z" => browser.next_block(true),
        b"/" => {
            browser.searching = true;
            browser.query.clear();
        }
        b"n" => browser.find(false, blocks),
        b"N" => browser.find(true, blocks),
        b"c" | b"\x03" => return copy(browser, blocks),
        b"r" => return run(browser, blocks, true),
        b"e" => return run(browser, blocks, false),
        b"f" | b" " => {
            if let Some(id) = browser.selected {
                blocks.toggle(id);
                return Action::Refresh;
            }
        }
        b"o" => return open(browser, blocks),
        _ => return Action::Send(bytes.to_vec()),
    }
    Action::None
}
pub(super) fn copy(browser: &Browser, blocks: &Blocks) -> Action {
    if let Some(text) = browser.selection.text(blocks) {
        return Action::Copy(text);
    }
    browser
        .selected
        .and_then(|id| blocks.get(id))
        .map_or(Action::None, |block| {
            Action::Copy(format!("$ {}\n{}", block.command, block.output))
        })
}
pub(super) fn run(browser: &Browser, blocks: &Blocks, execute: bool) -> Action {
    browser
        .selected
        .and_then(|id| blocks.get(id))
        .filter(|block| block.status.is_some())
        .map_or(Action::None, |block| {
            Action::Run(block.command.clone(), execute)
        })
}
pub(super) fn open(browser: &Browser, blocks: &Blocks) -> Action {
    let Some(id) = browser.selected else {
        return Action::None;
    };
    let Some(block) = blocks.get(id) else {
        return Action::None;
    };
    for (byte, _) in block
        .output
        .char_indices()
        .filter(|(byte, _)| *byte == 0 || block.output.as_bytes()[byte - 1].is_ascii_whitespace())
    {
        if let Some(path) = files::at(&block.output, byte, &block.cwd) {
            return Action::Open(path);
        }
    }
    Action::None
}
pub(super) fn point(row: &super::rows::Row, column: u16) -> Option<Point> {
    let (section, _) = row.source.as_ref()?;
    Some(Point {
        block: row.block,
        section: *section,
        byte: row.offset(column)?,
    })
}
pub(super) fn path(point: Point, blocks: &Blocks) -> Action {
    let Some(block) = blocks.get(point.block) else {
        return Action::None;
    };
    let source = match point.section {
        Section::Command => &block.command,
        Section::Output => &block.output,
    };
    files::at(source, point.byte, &block.cwd).map_or(Action::None, Action::Open)
}

fn search_input(bytes: &[u8], browser: &mut Browser, blocks: &mut Blocks) {
    match bytes {
        b"\r" | b"\n" => {
            browser.searching = false;
            browser.find(false, blocks);
        }
        b"\x7f" => {
            browser.query.pop();
        }
        _ => {
            if let Ok(text) = std::str::from_utf8(bytes) {
                browser
                    .query
                    .extend(text.chars().filter(|character| !character.is_control()));
            }
        }
    }
}
