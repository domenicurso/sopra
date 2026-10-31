use super::blocks::{Block, Blocks};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Section {
    Command,
    Output,
}
#[derive(Clone, Debug)]
pub(super) struct Row {
    pub(super) block: u64,
    pub(super) text: String,
    pub(super) source: Option<(Section, Range<usize>)>,
}
impl Row {
    pub(super) fn offset(&self, column: u16) -> Option<usize> {
        let (_, range) = self.source.as_ref()?;
        let mut cells = 2;
        for (offset, grapheme) in self.text.grapheme_indices(true) {
            if cells + usize::from(super::cells::width(grapheme)) > usize::from(column) {
                return Some(range.start + offset);
            }
            cells += usize::from(super::cells::width(grapheme));
        }
        Some(range.end)
    }
}

pub(super) fn layout(blocks: &Blocks, width: u16) -> Vec<Row> {
    let mut rows = Vec::new();
    for block in &blocks.entries {
        let status = block
            .status
            .map_or_else(|| "running".into(), |status| format!("exit {status}"));
        rows.push(Row {
            block: block.id,
            source: None,
            text: format!(
                "{} #{}  {} · {:.1}ms  [copy] [run]",
                if block.collapsed { '▶' } else { '▼' },
                block.id,
                status,
                block.duration.as_secs_f64() * 1000.0
            ),
        });
        append(&mut rows, block, Section::Command, width);
        if !block.collapsed {
            append(&mut rows, block, Section::Output, width);
        }
    }
    rows
}
fn append(rows: &mut Vec<Row>, block: &Block, section: Section, width: u16) {
    let source = match section {
        Section::Command => &block.command,
        Section::Output => &block.output,
    };
    if source.is_empty() {
        return;
    }
    let width = usize::from(width.saturating_sub(3).max(1));
    let mut start = 0;
    let mut cells = 0;
    for (offset, grapheme) in source.grapheme_indices(true) {
        if grapheme == "\n" || cells + usize::from(super::cells::width(grapheme)) > width {
            push(rows, block, section, start..offset);
            start = offset + if grapheme == "\n" { 1 } else { 0 };
            cells = 0;
        }
        if grapheme != "\n" {
            cells += usize::from(super::cells::width(grapheme));
        }
    }
    if start < source.len() {
        push(rows, block, section, start..source.len());
    }
}
fn push(rows: &mut Vec<Row>, block: &Block, section: Section, range: Range<usize>) {
    let source = match section {
        Section::Command => &block.command,
        Section::Output => &block.output,
    };
    let text = source[range.clone()].to_string();
    rows.push(Row {
        block: block.id,
        text,
        source: Some((section, range)),
    });
}
