#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Synopsis {
    Sequence(Vec<Synopsis>),
    Optional(Vec<Synopsis>),
    Choice(Vec<Vec<Synopsis>>),
    Repeat(Box<Synopsis>),
    Token(String),
}
