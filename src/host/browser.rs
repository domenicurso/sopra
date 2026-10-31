use super::{
    blocks::Blocks,
    rows::{self, Row},
    selection::Selection,
};

pub(super) struct Browser {
    pub(super) rows: Vec<Row>,
    pub(super) top: usize,
    pub(super) selected: Option<u64>,
    pub(super) query: String,
    pub(super) searching: bool,
    pub(super) selection: Selection,
    pub(super) notice: String,
    width: u16,
}
impl Browser {
    pub(super) fn new(blocks: &Blocks, width: u16, height: usize) -> Self {
        let mut browser = Self {
            rows: Vec::new(),
            top: 0,
            selected: blocks.entries.back().map(|block| block.id),
            query: String::new(),
            searching: false,
            selection: Selection::default(),
            notice: String::new(),
            width,
        };
        browser.refresh(blocks, width);
        browser.top = browser.rows.len().saturating_sub(height);
        browser
    }
    pub(super) fn refresh(&mut self, blocks: &Blocks, width: u16) {
        self.width = width;
        let anchor = self
            .rows
            .get(self.top)
            .map(|row| (row.block, row.source.clone()));
        self.rows = rows::layout(blocks, width);
        if let Some((block, source)) = anchor {
            self.top = self
                .rows
                .iter()
                .position(|row| {
                    row.block == block
                        && match (&source, &row.source) {
                            (Some((section, range)), Some((next_section, next))) => {
                                section == next_section && next.contains(&range.start)
                            }
                            (None, None) => true,
                            _ => false,
                        }
                })
                .unwrap_or(self.top);
        }
        self.top = self.top.min(self.rows.len().saturating_sub(1));
        if !self.selected.is_some_and(|id| blocks.get(id).is_some()) {
            self.selected = blocks.entries.back().map(|block| block.id);
        }
    }
    pub(super) fn scroll(&mut self, delta: isize, height: usize) {
        self.top = self
            .top
            .saturating_add_signed(delta)
            .min(self.rows.len().saturating_sub(height));
    }
    pub(super) fn next_block(&mut self, backwards: bool) {
        let headers = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.source.is_none())
            .map(|(index, row)| (index, row.block))
            .collect::<Vec<_>>();
        if headers.is_empty() {
            return;
        }
        let current = headers
            .iter()
            .position(|(_, id)| Some(*id) == self.selected)
            .unwrap_or(0);
        let index = if backwards {
            current.saturating_sub(1)
        } else {
            (current + 1).min(headers.len() - 1)
        };
        self.selected = Some(headers[index].1);
        self.top = headers[index].0;
        self.selection.clear();
    }
    pub(super) fn find(&mut self, backwards: bool, blocks: &mut Blocks) {
        if self.query.is_empty() {
            return;
        }
        let query = self.query.to_lowercase();
        for block in &mut blocks.entries {
            if block.collapsed
                && (block.command.to_lowercase().contains(&query)
                    || block.output.to_lowercase().contains(&query))
            {
                block.collapsed = false;
            }
        }
        self.refresh(blocks, self.width);
        let matches = super::search::matches(&self.rows, &self.query);
        let found = if backwards {
            matches
                .iter()
                .rev()
                .find(|index| **index < self.top)
                .or(matches.last())
        } else {
            matches
                .iter()
                .find(|index| **index > self.top)
                .or(matches.first())
        };
        if let Some(index) = found {
            self.top = *index;
            self.selected = Some(self.rows[*index].block);
            self.notice = format!("{} matches", matches.len());
        } else {
            self.notice = "No matches".into();
        }
    }
}
