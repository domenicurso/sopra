#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WordRole {
    Command,
    Keyword,
    Operator,
    Argument,
}

#[derive(Debug, Clone, Copy)]
enum Header {
    Loop,
    Case,
}

#[derive(Debug, Clone, Copy)]
enum CasePhase {
    Pattern,
    Body,
}

#[derive(Debug, Clone)]
pub(super) struct ShellState {
    pub(super) command_expected: bool,
    redirect_expected: bool,
    conditional: bool,
    header: Option<Header>,
    cases: Vec<CasePhase>,
}

impl ShellState {
    pub(super) const fn new() -> Self {
        Self {
            command_expected: true,
            redirect_expected: false,
            conditional: false,
            header: None,
            cases: Vec::new(),
        }
    }

    pub(super) fn consume_word(&mut self, word: &str) -> WordRole {
        if self.conditional {
            return conditional_role(word);
        }
        if self.redirect_expected {
            self.redirect_expected = false;
            return WordRole::Argument;
        }
        if let Some(header) = self.header {
            return self.consume_header(word, header);
        }
        if self.in_case_pattern() {
            if word == "esac" {
                self.cases.pop();
                self.command_expected = false;
                return WordRole::Keyword;
            }
            return WordRole::Argument;
        }
        if !self.command_expected || is_assignment(word) || word == "\\\n" {
            return WordRole::Argument;
        }
        if word == "!" {
            return WordRole::Operator;
        }
        self.command_expected = match word {
            "if" | "then" | "elif" | "else" | "while" | "until" | "do" => true,
            "fi" | "done" => false,
            "esac" => {
                self.cases.pop();
                false
            }
            "for" | "select" => {
                self.header = Some(Header::Loop);
                false
            }
            "case" => {
                self.header = Some(Header::Case);
                false
            }
            _ => {
                self.command_expected = false;
                return WordRole::Command;
            }
        };
        WordRole::Keyword
    }

    fn consume_header(&mut self, word: &str, header: Header) -> WordRole {
        match (header, word) {
            (Header::Loop, "do") => {
                self.header = None;
                self.command_expected = true;
                WordRole::Keyword
            }
            (Header::Case, "in") => {
                self.header = None;
                self.command_expected = false;
                self.cases.push(CasePhase::Pattern);
                WordRole::Keyword
            }
            (Header::Loop, "in") => WordRole::Keyword,
            _ => WordRole::Argument,
        }
    }

    pub(super) fn in_case_pattern(&self) -> bool {
        matches!(self.cases.last(), Some(CasePhase::Pattern))
    }

    pub(super) fn consume_operator(&mut self, operator: &str) {
        match operator {
            "[" | "[[" if self.command_expected => self.conditional = true,
            "]" | "]]" | ";" => self.conditional = false,
            _ => {}
        }
        if self.in_case_pattern() && operator == ")" {
            if let Some(phase) = self.cases.last_mut() {
                *phase = CasePhase::Body;
            }
        } else if matches!(operator, ";;" | ";&" | ";;&")
            && let Some(phase) = self.cases.last_mut()
        {
            *phase = CasePhase::Pattern;
        }
        self.redirect_expected = operator
            .chars()
            .any(|character| matches!(character, '<' | '>'));
        if matches!(
            operator,
            "|" | "||" | "&&" | ";" | ";&" | ";;" | ";;&" | "&" | "(" | "{" | ")"
        ) {
            self.command_expected = true;
        } else if !self.redirect_expected {
            self.command_expected = false;
        }
    }
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character.is_ascii_alphabetic()
                || character.is_ascii_digit() && index > 0
        })
}

fn conditional_role(word: &str) -> WordRole {
    if matches!(
        word,
        "!" | "="
            | "=="
            | "!="
            | "=~"
            | "-a"
            | "-b"
            | "-c"
            | "-d"
            | "-e"
            | "-f"
            | "-g"
            | "-h"
            | "-k"
            | "-L"
            | "-n"
            | "-o"
            | "-p"
            | "-r"
            | "-s"
            | "-S"
            | "-t"
            | "-u"
            | "-w"
            | "-x"
            | "-z"
            | "-O"
            | "-G"
            | "-N"
            | "-nt"
            | "-ot"
            | "-ef"
            | "-eq"
            | "-ne"
            | "-lt"
            | "-le"
            | "-gt"
            | "-ge"
    ) {
        WordRole::Operator
    } else {
        WordRole::Argument
    }
}
