use super::{Document, Section, append, sections};

pub(super) fn extract(text: &str) -> Document {
    let mut scanner = Scanner {
        document: Document {
            preamble: Vec::new(),
            usage: Vec::new(),
            commands: Vec::new(),
            options: Vec::new(),
            positionals: Vec::new(),
        },
        section: Section::Other,
        usage: Usage::default(),
        rows: append::State::new(),
        preamble: true,
        heading_indent: None,
    };
    for raw in text.lines().map(str::trim_end) {
        scanner.line(raw);
    }
    scanner.document
}

struct Scanner {
    document: Document,
    section: Section,
    usage: Usage,
    rows: append::State,
    preamble: bool,
    heading_indent: Option<usize>,
}

impl Scanner {
    fn line(&mut self, raw: &str) {
        if let Some(rest) = sections::usage_line(raw) {
            self.preamble = false;
            self.heading_indent = Some(raw.len() - raw.trim_start().len());
            self.usage = Usage {
                open: true,
                empty_header: rest.trim().is_empty(),
                seen: !rest.trim().is_empty(),
            };
            self.rows.reset("");
            append_usage(&mut self.document.usage, rest, true);
            return;
        }
        if let Some(next) = sections::section_header(raw)
            && self
                .heading_indent
                .is_none_or(|indent| raw.len() - raw.trim_start().len() <= indent)
        {
            self.heading_indent = Some(raw.len() - raw.trim_start().len());
            self.preamble = false;
            self.usage = Usage::default();
            self.rows.reset(raw);
            self.section = next;
            return;
        }
        if self.usage.append(&mut self.document.usage, raw) || raw.trim().is_empty() {
            return;
        }
        if self.rows.append(&mut self.document, self.section, raw) {
            self.preamble = false;
            return;
        }
        if self.preamble {
            self.document.preamble.push(raw.trim().to_string());
        }
    }
}

#[derive(Default)]
struct Usage {
    open: bool,
    empty_header: bool,
    seen: bool,
}

impl Usage {
    fn append(&mut self, target: &mut Vec<String>, raw: &str) -> bool {
        if !self.open {
            return false;
        }
        if raw.trim().is_empty() {
            self.open = self.empty_header && !self.seen;
            return true;
        }
        if raw.starts_with([' ', '\t'])
            || sections::looks_like_usage(raw)
            || (self.empty_header && !self.seen)
        {
            let value = raw.trim();
            let new_line = usage_starts_new_form(target, value);
            append_usage(target, value, new_line);
            self.seen = true;
            return true;
        }
        self.open = false;
        false
    }
}

fn append_usage(target: &mut Vec<String>, value: &str, new_line: bool) {
    let value = usage_form(value);
    if value.is_empty() {
        return;
    }
    if new_line || target.is_empty() {
        target.push(value.to_string());
    } else if let Some(last) = target.last_mut() {
        if !last.is_empty() {
            last.push(' ');
        }
        last.push_str(value);
    }
}

fn usage_starts_new_form(existing: &[String], value: &str) -> bool {
    let Some(first) = existing
        .first()
        .and_then(|usage| usage.split_whitespace().next())
    else {
        return true;
    };
    usage_form(value).split_whitespace().next() == Some(first)
}

fn usage_form(value: &str) -> &str {
    value
        .trim()
        .strip_prefix("or:")
        .map_or(value.trim(), str::trim_start)
}
