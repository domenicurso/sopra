use super::{Document, Row, rows};

pub(super) struct State {
    parents: Vec<usize>,
}

impl State {
    pub(super) fn new() -> Self {
        Self {
            parents: Vec::new(),
        }
    }

    pub(super) fn reset(&mut self) {
        self.parents.clear();
    }

    pub(super) fn append(&mut self, document: &mut Document, raw: &str) -> bool {
        let Some((depth, content)) = decode(raw) else {
            return false;
        };
        let Some(root) = document.commands.last_mut() else {
            return false;
        };
        if depth > self.parents.len() {
            return false;
        }
        self.parents.truncate(depth);
        let (left, description) = rows::split_columns(&content)
            .unwrap_or_else(|| (content.trim().to_string(), String::new()));
        let child = Row {
            indent: raw.len() - raw.trim_start().len(),
            left,
            description,
            columns: rows::columns(&content),
            children: Vec::new(),
        };
        let Some(index) = append_child(root, &self.parents, child) else {
            return false;
        };
        self.parents.push(index);
        true
    }
}

fn decode(raw: &str) -> Option<(usize, String)> {
    let (index, marker) = raw
        .char_indices()
        .find(|(_, character)| branch_marker(*character))?;
    let prefix = &raw[..index];
    let content = raw[index + marker.len_utf8()..]
        .trim_start_matches([' ', '─', '━'])
        .trim()
        .to_string();
    (!content.is_empty()).then_some((branch_depth(prefix), content))
}

fn branch_marker(character: char) -> bool {
    matches!(character, '├' | '└' | '╰' | '┣' | '┗')
}

fn branch_depth(prefix: &str) -> usize {
    prefix
        .chars()
        .filter(|character| matches!(*character, '│' | '┃'))
        .count()
}

fn append_child(root: &mut Row, path: &[usize], child: Row) -> Option<usize> {
    let parent = descend(root, path)?;
    parent.children.push(child);
    Some(parent.children.len() - 1)
}

fn descend<'a>(row: &'a mut Row, path: &[usize]) -> Option<&'a mut Row> {
    let Some(index) = path.first() else {
        return Some(row);
    };
    descend(row.children.get_mut(*index)?, &path[1..])
}
