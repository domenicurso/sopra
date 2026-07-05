use keel_core::{PromptConfig, PromptSpanStyle, PromptSurface, PromptToken, ShellSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptLayout {
    pub active_left: PromptSurface,
    pub active_right: PromptSurface,
    pub transient_left: PromptSurface,
}

#[derive(Debug, Default)]
pub struct PromptRuntime;

impl PromptRuntime {
    pub fn layout(&self, config: &PromptConfig, shell: &ShellSnapshot) -> PromptLayout {
        PromptLayout {
            active_left: self.render_tokens(&config.active_left, shell, PromptSpanStyle::Prompt),
            active_right: self.render_tokens(&config.active_right, shell, PromptSpanStyle::Muted),
            transient_left: self.render_tokens(
                &config.transient_left,
                shell,
                PromptSpanStyle::Prompt,
            ),
        }
    }

    fn render_tokens(
        &self,
        tokens: &[PromptToken],
        shell: &ShellSnapshot,
        default_style: PromptSpanStyle,
    ) -> PromptSurface {
        let mut rendered = PromptSurface::default();

        for token in tokens {
            match token {
                PromptToken::Literal(text) => rendered.push(text.clone(), default_style),
                PromptToken::CurrentDirectory => rendered.push(shell.cwd.clone(), default_style),
                PromptToken::ExitStatus => {
                    if shell.last_status != 0 {
                        rendered.push(
                            format!("[{}]", shell.last_status),
                            PromptSpanStyle::StatusError,
                        );
                    }
                }
                PromptToken::CommandDuration => {
                    if let Some(duration_ms) = shell.last_duration_ms {
                        rendered.push(
                            format!(" {}ms", duration_ms),
                            PromptSpanStyle::Muted,
                        );
                    }
                }
                PromptToken::Widget(name) => {
                    rendered.push(format!("{{{name}}}"), PromptSpanStyle::Accent);
                }
            }
        }

        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::PromptRuntime;
    use keel_core::{PromptConfig, PromptToken, ShellSnapshot};

    #[test]
    fn renders_active_and_transient_prompt_variants() {
        let runtime = PromptRuntime;
        let layout = runtime.layout(
            &PromptConfig {
                active_left: vec![
                    PromptToken::CurrentDirectory,
                    PromptToken::Literal(" > ".to_string()),
                ],
                active_right: vec![PromptToken::ExitStatus],
                transient_left: vec![PromptToken::Literal("$ ".to_string())],
            },
            &ShellSnapshot {
                cwd: "/tmp/demo".to_string(),
                last_status: 7,
                last_duration_ms: Some(12),
                aliases: Vec::new(),
                functions: Vec::new(),
                environment: Vec::new(),
            },
        );

        assert_eq!(layout.active_left.plain_text(), "/tmp/demo > ");
        assert_eq!(layout.active_right.plain_text(), "[7]");
        assert_eq!(layout.transient_left.plain_text(), "$ ");
        assert_eq!(
            layout.active_right.spans[0].style,
            keel_core::PromptSpanStyle::StatusError
        );
    }
}
