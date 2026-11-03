use super::entries::{
    command_context, command_list, continuation, indentation, is_new_row, last_row_mut, push_row,
};
use super::rows;
use super::rows::{
    append_description, command_description_continuation, command_grid, row_allowed, split_columns,
    unlabelled_row,
};
use super::{Document, Section, options, sections, tree};

pub(super) struct State {
    tree: tree::State,
    seen: bool,
    group: bool,
    unsectioned_option: bool,
    description_column: Option<usize>,
}

impl State {
    pub(super) fn new() -> Self {
        Self {
            tree: tree::State::new(),
            seen: false,
            group: false,
            unsectioned_option: false,
            description_column: None,
        }
    }

    pub(super) fn reset(&mut self, raw: &str) {
        self.tree.reset();
        self.seen = false;
        self.unsectioned_option = false;
        self.description_column = None;
        self.group = sections::is_command_group_header(raw);
    }

    pub(super) fn append(&mut self, document: &mut Document, section: Section, raw: &str) -> bool {
        self.special_row(document, section, raw)
            || self.table_row(document, section, raw)
            || self.option_continuation(document, section, raw)
            || continuation(document, section, raw)
    }

    fn option_continuation(&self, document: &mut Document, section: Section, raw: &str) -> bool {
        if section == Section::Other
            && self.unsectioned_option
            && let Some(row) = document.options.last_mut()
            && indentation(raw) > row.indent
        {
            append_description(row, raw);
            return true;
        }
        false
    }

    fn special_row(&mut self, document: &mut Document, section: Section, raw: &str) -> bool {
        if section == Section::Commands && self.tree.append(document, raw) {
            return true;
        }
        if self.group_row(document, section, raw) {
            return true;
        }
        if section == Section::Commands
            && !self.seen
            && !rows::command_row(raw)
            && let Some(row) = document.options.last_mut()
            && indentation(raw) > row.indent
        {
            append_description(row, raw);
            return true;
        }
        if section == Section::Commands && options::append(document, raw) {
            return true;
        }
        if section == Section::Commands
            && command_description_continuation(raw, document.commands.last())
        {
            let Some(row) = document.commands.last_mut() else {
                return false;
            };
            append_description(row, raw);
            return true;
        }
        false
    }

    fn group_row(&mut self, document: &mut Document, section: Section, raw: &str) -> bool {
        if section == Section::Other
            && self.group
            && command_context(document)
            && let Some((left, description)) = rows::command_table_row(raw)
        {
            self.tree.reset();
            push_row(document, Section::Commands, raw, (left, description));
            self.seen = true;
            return true;
        }
        if section == Section::Other
            && self.group
            && self.seen
            && let Some(row) = document.commands.last_mut()
            && indentation(raw) > row.indent
        {
            append_description(row, raw);
            return true;
        }
        false
    }

    fn overflow_row(
        &self,
        document: &mut Document,
        section: Section,
        raw: &str,
    ) -> Option<(String, String)> {
        if section != Section::Commands {
            return None;
        }
        let previous = document.commands.last()?;
        if indentation(raw) != previous.indent {
            return None;
        }
        let (name, description) = raw.trim().split_once(char::is_whitespace)?;
        (indentation(raw) + name.len() + 1 >= self.description_column?)
            .then(|| (name.to_string(), description.trim().to_string()))
    }

    fn name_list(&mut self, document: &mut Document, section: Section, raw: &str) -> bool {
        if section != Section::Commands {
            return false;
        }
        let names = command_grid(raw).or_else(|| {
            command_list(raw).then(|| {
                raw.trim()
                    .trim_end_matches(',')
                    .split(',')
                    .map(|name| name.trim().to_string())
                    .collect()
            })
        });
        let Some(names) = names else {
            return false;
        };
        self.tree.reset();
        for name in names {
            push_row(document, Section::Commands, raw, (name, String::new()));
            if let Some(row) = document.commands.last_mut() {
                row.columns = vec![row.left.clone()];
            }
        }
        self.seen = true;
        true
    }

    fn table_row(&mut self, document: &mut Document, section: Section, raw: &str) -> bool {
        if self.name_list(document, section, raw) {
            return true;
        }
        if matches!(section, Section::Options | Section::Other) && options::append(document, raw) {
            self.unsectioned_option = section == Section::Other;
            return true;
        }
        if is_new_row(document, section, raw)
            && let Some((left, description)) = split_columns(raw)
                .or_else(|| self.overflow_row(document, section, raw))
                .or_else(|| {
                    let previous = last_row_mut(document, section);
                    if previous.is_some_and(|row| indentation(raw) > row.indent) {
                        None
                    } else {
                        unlabelled_row(raw, section)
                    }
                })
            && row_allowed(section, &left)
        {
            if section == Section::Commands {
                self.tree.reset();
                self.seen = true;
            }
            if section == Section::Commands && !description.is_empty() {
                self.description_column = raw.find(&description);
            }
            push_row(document, section, raw, (left, description));
            return true;
        }
        false
    }
}
