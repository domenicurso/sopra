use super::{
    blocks::Blocks,
    browser::Browser,
    rows::{self, Section},
    selection::{Point, Selection},
};
use crate::config::SessionConfig;
use portable_pty::PtySize;
use std::io;

fn blocks() -> Blocks {
    Blocks::new(SessionConfig {
        max_blocks: 2,
        scrollback_lines: 100,
        max_output_bytes: 4096,
    })
}
#[test]
fn selection_remains_on_logical_unicode_text_after_reflow_and_multiclick() -> io::Result<()> {
    let mut blocks = blocks();
    blocks.begin("echo example".into(), "/tmp".into(), PtySize::default());
    blocks.process("hello café world\r\nsecond line\r\n".as_bytes());
    blocks.finish(0)?;
    let point = Point {
        block: 1,
        section: Section::Output,
        byte: 7,
    };
    let mut selection = Selection::default();
    selection.press(point, &blocks);
    selection.press(point, &blocks);
    assert_eq!(selection.text(&blocks).as_deref(), Some("café"));
    for width in [120, 40, 8, 120] {
        let rows = rows::layout(&blocks, width);
        let selected = rows
            .iter()
            .filter_map(|row| selection.range(row).and_then(|range| row.text.get(range)))
            .collect::<String>();
        assert_eq!(selected, "café");
    }
    selection.press(point, &blocks);
    assert_eq!(selection.text(&blocks).as_deref(), Some("hello café world"));
    Ok(())
}
#[test]
fn retention_evicts_old_blocks_and_output_limits_are_explicit() -> io::Result<()> {
    let mut blocks = blocks();
    for command in ["one", "two", "three"] {
        blocks.begin(command.into(), "/tmp".into(), PtySize::default());
        blocks.process(b"result\r\n");
        blocks.finish(0)?;
    }
    assert_eq!(blocks.entries.len(), 2);
    assert_eq!(blocks.entries[0].command, "two");
    blocks.begin("large".into(), "/tmp".into(), PtySize::default());
    blocks.process(&vec![b'x'; 8192]);
    blocks.finish(0)?;
    assert!(
        blocks
            .entries
            .back()
            .is_some_and(|block| block.output.contains("output truncated"))
    );
    Ok(())
}
#[test]
fn folding_and_search_preserve_block_navigation() -> io::Result<()> {
    let mut blocks = blocks();
    blocks.begin("first".into(), "/tmp".into(), PtySize::default());
    blocks.process(b"searchable-output\r\n");
    blocks.finish(3)?;
    let mut browser = Browser::new(&blocks, 12, 10);
    browser.query = "searchable-output".into();
    browser.find(false, &mut blocks);
    assert_eq!(browser.notice, "1 matches");
    blocks.toggle(1);
    browser.refresh(&blocks, 12);
    assert!(browser.rows.iter().all(|row| {
        row.source
            .as_ref()
            .is_none_or(|(section, _)| *section != Section::Output)
    }));
    assert_eq!(browser.selected, Some(1));
    browser.find(false, &mut blocks);
    assert!(!blocks.entries[0].collapsed);
    assert_eq!(browser.notice, "1 matches");
    Ok(())
}
#[test]
fn terminal_queries_use_child_coordinates_and_preserve_forwarded_controls() -> io::Result<()> {
    let mut model = super::model::Model::new(
        PtySize::default(),
        SessionConfig {
            max_blocks: 2,
            scrollback_lines: 100,
            max_output_bytes: 4096,
        },
    );
    model.process(b"\x1b[3;9H\x1b[6n\x1b[18t\x1b]11;?\x07\x1b]2;title\x07")?;
    let callbacks = model.terminal.callbacks_mut();
    assert!(callbacks.replies.starts_with(b"\x1b[3;9R"));
    assert!(
        callbacks
            .terminal
            .windows(6)
            .any(|bytes| bytes == b"11;?\x07\x1b")
    );
    Ok(())
}

#[test]
fn missing_command_metadata_does_not_leave_a_foreground_program_editable() -> io::Result<()> {
    let mut model = super::model::Model::new(
        PtySize::default(),
        SessionConfig {
            max_blocks: 2,
            scrollback_lines: 100,
            max_output_bytes: 4096,
        },
    );
    model.process(b"\x1b]777;sopra;executing\x07\x1b]777;sopra;begin;invalid;00\x07")?;
    assert!(model.executing && model.prompt.is_none());
    model.process(b"\x1b]777;sopra;end;0\x07\x1b]777;sopra;editing;cont\x07")?;
    assert!(!model.executing && model.secondary);
    assert!(model.blocks.entries.is_empty());
    Ok(())
}
