mod options;
mod parse;
mod rows;
mod sections;
mod tree;

pub(crate) use sections::Section;

#[derive(Clone)]
pub(crate) struct Row {
    pub(crate) indent: usize,
    pub(crate) left: String,
    pub(crate) description: String,
    pub(crate) columns: Vec<String>,
    pub(crate) children: Vec<Row>,
}

pub(crate) struct Document {
    pub(crate) usage: Vec<String>,
    pub(crate) commands: Vec<Row>,
    pub(crate) options: Vec<Row>,
    pub(crate) positionals: Vec<Row>,
}

pub(crate) fn extract(text: &str) -> Document {
    parse::extract(text)
}
