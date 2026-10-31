use super::{
    blocks::Blocks,
    rows::{Row, Section},
};
use std::{
    ops::Range,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Point {
    pub(super) block: u64,
    pub(super) section: Section,
    pub(super) byte: usize,
}
#[derive(Default)]
pub(super) struct Selection {
    anchor: Option<Point>,
    focus: Option<Point>,
    previous: Option<(Point, Instant, u8)>,
    unit: Option<(Point, Point)>,
}
impl Selection {
    pub(super) fn press(&mut self, point: Point, blocks: &Blocks) {
        let count = self
            .previous
            .filter(|(previous, time, _)| {
                *previous == point && time.elapsed() < Duration::from_millis(400)
            })
            .map_or(1, |(_, _, count)| count % 3 + 1);
        self.previous = Some((point, Instant::now(), count));
        let range = source(blocks, point).map_or(point.byte..point.byte, |source| {
            unit(source, point.byte, count)
        });
        self.anchor = Some(Point {
            byte: range.start,
            ..point
        });
        self.focus = Some(Point {
            byte: range.end,
            ..point
        });
        self.unit = Some((
            Point {
                byte: range.start,
                ..point
            },
            Point {
                byte: range.end,
                ..point
            },
        ));
    }
    pub(super) fn drag(&mut self, point: Point) {
        if let Some((start, end)) = self.unit {
            self.anchor = Some(if point < start { end } else { start });
            self.focus = Some(point);
        }
    }
    pub(super) fn clear(&mut self) {
        self.anchor = None;
        self.focus = None;
        self.unit = None;
    }
    pub(super) fn invalidate(&mut self, block: u64) {
        if self.anchor.is_some_and(|point| point.block == block)
            || self.focus.is_some_and(|point| point.block == block)
        {
            self.clear();
        }
    }
    fn bounds(&self) -> Option<(Point, Point)> {
        let (a, b) = (self.anchor?, self.focus?);
        (a != b).then_some((a.min(b), a.max(b)))
    }
    pub(super) fn range(&self, row: &Row) -> Option<Range<usize>> {
        let (section, range) = row.source.as_ref()?;
        let (start, end) = self.bounds()?;
        let key = (row.block, *section);
        if key < (start.block, start.section) || key > (end.block, end.section) {
            return None;
        }
        let from = if key == (start.block, start.section) {
            start.byte.max(range.start)
        } else {
            range.start
        };
        let to = if key == (end.block, end.section) {
            end.byte.min(range.end)
        } else {
            range.end
        };
        (from < to).then_some(from.saturating_sub(range.start)..to.saturating_sub(range.start))
    }
    pub(super) fn text(&self, blocks: &Blocks) -> Option<String> {
        let (start, end) = self.bounds()?;
        let mut pieces = Vec::new();
        for block in &blocks.entries {
            for (section, text) in [
                (Section::Command, &block.command),
                (Section::Output, &block.output),
            ] {
                let key = (block.id, section);
                if key < (start.block, start.section) || key > (end.block, end.section) {
                    continue;
                }
                let from = if key == (start.block, start.section) {
                    start.byte
                } else {
                    0
                };
                let to = if key == (end.block, end.section) {
                    end.byte
                } else {
                    text.len()
                };
                if let Some(piece) = text.get(from..to) {
                    pieces.push(piece);
                }
            }
        }
        Some(pieces.join("\n"))
    }
}
fn source(blocks: &Blocks, point: Point) -> Option<&str> {
    let block = blocks.get(point.block)?;
    Some(match point.section {
        Section::Command => &block.command,
        Section::Output => &block.output,
    })
}
fn unit(source: &str, offset: usize, count: u8) -> Range<usize> {
    if count == 1 {
        return offset..offset;
    }
    if count == 2 {
        return source
            .split_word_bound_indices()
            .find(|(start, text)| (*start..start + text.len()).contains(&offset))
            .map_or(offset..offset, |(start, text)| start..start + text.len());
    }
    let start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let end = source[offset..]
        .find('\n')
        .map_or(source.len(), |index| offset + index);
    start..end
}
