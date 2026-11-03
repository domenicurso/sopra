use super::Synopsis;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CommandGraph {
    pub(crate) root: CommandNode,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CommandNode {
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) description: Option<String>,
    pub(crate) options: Vec<OptionSpec>,
    pub(crate) positionals: Vec<PositionalSpec>,
    pub(crate) subcommands: Vec<CommandNode>,
    pub(crate) synopsis: Option<Synopsis>,
    pub(crate) exclusive: Vec<Vec<String>>,
    pub(crate) evidence: Vec<super::evidence::Evidence>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionSpec {
    pub(crate) names: Vec<String>,
    pub(crate) description: Option<String>,
    pub(crate) value: Option<ValueSpec>,
    pub(crate) attachment: ValueAttachment,
    pub(crate) repeatable: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ValueAttachment {
    #[default]
    Separate,
    Attached,
    Either,
    Equals,
    Colon,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct PositionalSpec {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) value: ValueSpec,
    pub(crate) optional: bool,
    pub(crate) repeatable: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValueSpec {
    pub(crate) name: String,
    pub(crate) kind: ValueKind,
    pub(crate) choices: Vec<String>,
    pub(crate) optional: bool,
    pub(crate) literal: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ValueKind {
    #[default]
    Text,
    Number,
    File,
    Directory,
    Url,
}
