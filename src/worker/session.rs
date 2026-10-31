mod controls;
use crate::{
    arguments::Args,
    config::Config,
    editor::{EditorConfig, EditorState, History, RunResult},
    input::{CursorPosition, Terminal},
    palette::TerminalPalette,
    prompt::widgets::Widgets,
};
use sopra_completion::CompletionEngine;
use std::error::Error;

pub(super) struct Session {
    config: Config,
    config_path: std::path::PathBuf,
    cwd: std::path::PathBuf,
    palette: Option<TerminalPalette>,
    widgets: Option<Widgets>,
    completion: Option<CompletionEngine>,
    history: History,
    syntax: crate::editor::submission::Checker,
}

impl Session {
    pub(super) fn new() -> Result<Self, Box<dyn Error>> {
        let config_path = crate::config::path()?;
        let config = Config::read_from(&config_path)?;
        let completion = config.completion.then(CompletionEngine::new).flatten();
        Ok(Self {
            config,
            config_path,
            cwd: std::env::current_dir()?,
            completion,
            palette: None,
            widgets: None,
            history: History::default(),
            syntax: crate::editor::submission::Checker::default(),
        })
    }

    pub(super) fn run(&mut self, args: Args) -> Result<RunResult, Box<dyn Error>> {
        let mut terminal = Terminal::open(self.palette)?;
        self.palette = Some(terminal.palette());
        self.cwd = args.cwd.clone();
        let secondary = args.secondary;
        let (widgets, primary) = self.widgets_for_edit(&args)?;
        self.history
            .refresh(args.history_file.as_deref(), &self.config)?;
        let cursor = args
            .buffer
            .char_indices()
            .nth(args.cursor)
            .map_or(args.buffer.len(), |(byte, _)| byte);
        let mut editor = EditorState::initialize(
            self.editor_config(args, &terminal)?,
            widgets,
            cursor,
            self.completion.take(),
        );
        editor.set_history(std::mem::take(&mut self.history));
        editor.set_transient_prompt(if secondary {
            self.config.secondary.clone()
        } else {
            self.config.transient.clone()
        });
        editor.set_secondary_prompt(self.config.secondary.clone());
        let result = editor.run(&mut terminal, &mut self.syntax);
        let (widgets, completion, history) = editor.into_services();
        self.widgets = Some(primary.unwrap_or(widgets));
        self.completion = completion;
        self.history = history;
        terminal.restore()?;
        result.map_err(Into::into)
    }

    fn widgets_for_edit(&mut self, args: &Args) -> std::io::Result<(Widgets, Option<Widgets>)> {
        let mut primary = match self.widgets.take() {
            Some(widgets) => widgets,
            None => Widgets::configured(
                self.config.left.clone(),
                self.config.right.clone(),
                args.cwd.clone(),
                &self.config.widgets,
            )?,
        };
        primary.update(&args.cwd, &self.config.widgets);
        if args.secondary {
            Ok((
                Widgets::configured(
                    self.config.secondary.clone(),
                    String::new(),
                    args.cwd.clone(),
                    &self.config.widgets,
                )?,
                Some(primary),
            ))
        } else {
            Ok((primary, None))
        }
    }

    fn editor_config(&self, args: Args, terminal: &Terminal) -> std::io::Result<EditorConfig> {
        Ok(EditorConfig {
            buffer: args.buffer,
            #[cfg(test)]
            cursor_chars: args.cursor,
            arm_completion: args.arm_completion,
            suppress_completion: args.suppress_completion,
            completion_enabled: self.config.completion,
            syntax_highlighting: self.config.syntax_highlighting,
            cursor_animation: self.config.cursor_animation,
            #[cfg(test)]
            prompt: self.config.left.clone(),
            #[cfg(test)]
            rprompt: self.config.right.clone(),
            anchor: CursorPosition { row: 0, column: 0 },
            size: terminal.size()?,
            cwd: args.cwd,
            palette: terminal.palette(),
        })
    }

    pub(super) fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    pub(super) fn shell_prompts(&self) -> std::io::Result<(String, String)> {
        Ok((
            crate::prompt::shell::render(&self.config.transient)?,
            crate::prompt::shell::render(&self.config.secondary)?,
        ))
    }
}
