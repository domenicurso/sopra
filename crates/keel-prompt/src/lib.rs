use keel_core::{PromptConfig, PromptToken, ShellSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptLayout {
    pub active_left: String,
    pub active_right: String,
    pub transient_left: String,
}

#[derive(Debug, Default)]
pub struct PromptRuntime;

impl PromptRuntime {
    pub fn layout(&self, config: &PromptConfig, shell: &ShellSnapshot) -> PromptLayout {
        PromptLayout {
            active_left: self.render_tokens(&config.active_left, shell),
            active_right: self.render_tokens(&config.active_right, shell),
            transient_left: self.render_tokens(&config.transient_left, shell),
        }
    }

    fn render_tokens(&self, tokens: &[PromptToken], shell: &ShellSnapshot) -> String {
        let mut rendered = String::new();

        for token in tokens {
            match token {
                PromptToken::Literal(text) => rendered.push_str(text),
                PromptToken::CurrentDirectory => rendered.push_str(&shell.cwd),
                PromptToken::ExitStatus => {
                    if shell.last_status != 0 {
                        rendered.push_str(&format!("[{}]", shell.last_status));
                    }
                }
                PromptToken::CommandDuration => {
                    if let Some(duration_ms) = shell.last_duration_ms {
                        rendered.push_str(&format!(" {}ms", duration_ms));
                    }
                }
                PromptToken::Widget(name) => {
                    rendered.push('{');
                    rendered.push_str(name);
                    rendered.push('}');
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

        assert_eq!(layout.active_left, "/tmp/demo > ");
        assert_eq!(layout.active_right, "[7]");
        assert_eq!(layout.transient_left, "$ ");
    }
}
