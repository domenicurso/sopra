#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingKind {
    Text,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingKey {
    Prompt,
    RightPrompt,
    TransientPrompt,
    CursorAnimation,
    SyntaxHighlighting,
    CompletionEnabled,
}

impl SettingKey {
    pub(super) const ALL: [Self; 6] = [
        Self::Prompt,
        Self::RightPrompt,
        Self::TransientPrompt,
        Self::CursorAnimation,
        Self::SyntaxHighlighting,
        Self::CompletionEnabled,
    ];

    pub(super) fn storage_name(self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::RightPrompt => "rprompt",
            Self::TransientPrompt => "transient_prompt",
            Self::CursorAnimation => "cursor_animation",
            Self::SyntaxHighlighting => "syntax_highlighting",
            Self::CompletionEnabled => "completion_enabled",
        }
    }

    pub(super) fn environment_name(self) -> &'static str {
        match self {
            Self::Prompt => "SOPRA_PROMPT",
            Self::RightPrompt => "SOPRA_RPROMPT",
            Self::TransientPrompt => "SOPRA_TRANSIENT_PROMPT",
            Self::CursorAnimation => "SOPRA_CURSOR_ANIMATION",
            Self::SyntaxHighlighting => "SOPRA_SYNTAX_HIGHLIGHTING",
            Self::CompletionEnabled => "SOPRA_COMPLETION_ENABLED",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SettingDefinition {
    pub(super) key: SettingKey,
    pub(super) label: &'static str,
    pub(super) description: &'static str,
    pub(super) kind: SettingKind,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TabDefinition {
    pub(super) label: &'static str,
    pub(super) description: &'static str,
    pub(super) settings: &'static [SettingDefinition],
}

const PROMPT_SETTINGS: &[SettingDefinition] = &[
    SettingDefinition {
        key: SettingKey::Prompt,
        label: "Prompt",
        description: "Prompt shown while editing a command.",
        kind: SettingKind::Text,
    },
    SettingDefinition {
        key: SettingKey::RightPrompt,
        label: "Right prompt",
        description: "Prompt rendered against the right edge of the line.",
        kind: SettingKind::Text,
    },
    SettingDefinition {
        key: SettingKey::TransientPrompt,
        label: "Transient prompt",
        description: "Prompt left after a line is accepted or interrupted.",
        kind: SettingKind::Text,
    },
];

const EDITOR_SETTINGS: &[SettingDefinition] = &[
    SettingDefinition {
        key: SettingKey::CursorAnimation,
        label: "Cursor animation",
        description: "Animate the editing cursor while the line is active.",
        kind: SettingKind::Toggle,
    },
    SettingDefinition {
        key: SettingKey::SyntaxHighlighting,
        label: "Syntax highlighting",
        description: "Color commands, strings, variables, and delimiters as you type.",
        kind: SettingKind::Toggle,
    },
];

const COMPLETION_SETTINGS: &[SettingDefinition] = &[SettingDefinition {
    key: SettingKey::CompletionEnabled,
    label: "Completions",
    description: "Offer command, option, value, and path suggestions while typing.",
    kind: SettingKind::Toggle,
}];

pub(super) const TABS: &[TabDefinition] = &[
    TabDefinition {
        label: "Prompts",
        description: "Control the prompts used by the Sopra shell bridge.",
        settings: PROMPT_SETTINGS,
    },
    TabDefinition {
        label: "Editor",
        description: "Tune the live line editor presentation.",
        settings: EDITOR_SETTINGS,
    },
    TabDefinition {
        label: "Completion",
        description: "Choose whether Sopra offers contextual suggestions.",
        settings: COMPLETION_SETTINGS,
    },
];
