#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Emphasis {
    #[default]
    Normal,
    Dim,
    Bold,
    Selected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub text: String,
    pub emphasis: Emphasis,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    pub parts: Vec<Part>,
}

impl Text {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(mut self, text: impl Into<String>) -> Self {
        self.parts.push(Part {
            text: text.into(),
            emphasis: Emphasis::Normal,
        });
        self
    }

    pub fn part(mut self, emphasis: Emphasis, text: impl Into<String>) -> Self {
        self.parts.push(Part {
            text: text.into(),
            emphasis,
        });
        self
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Self::new().push(text)
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        Self::new().push(text)
    }
}
