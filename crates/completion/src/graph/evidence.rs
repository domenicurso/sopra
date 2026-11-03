#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Evidence {
    pub(crate) target: String,
    pub(crate) source: String,
    pub(crate) line: usize,
    pub(crate) text: String,
    pub(crate) observation: String,
}
