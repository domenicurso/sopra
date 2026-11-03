use super::Session;
use crate::{
    config::Config,
    control::Command,
    prompt::widgets::{Widgets, inspection},
};
use sopra_completion::CompletionEngine;
use std::io;

impl Session {
    pub(in crate::worker) fn control(&mut self, command: Command) -> io::Result<String> {
        match command {
            Command::Refresh => {
                self.refresh()?;
                Ok("Sopra refreshed.\n".into())
            }
            Command::Restart => {
                Config::read_from(&self.config_path)?;
                Ok("Restarting Sopra.\n".into())
            }
            Command::CacheStatus => sopra_completion::cache::status(),
            Command::CacheClear(command) => self.clear_cache(command.as_deref()),
            Command::ConfigShow => Ok(self.config.effective()),
            Command::WidgetsList => inspection::list(&self.config.widgets, self.widgets.as_ref()),
            Command::Ping => Err(io::Error::other("health requests belong to the listener")),
        }
    }

    fn clear_cache(&mut self, command: Option<&str>) -> io::Result<String> {
        // Joining discovery before deletion prevents an old probe from restoring cleared files.
        drop(self.completion.take());
        let result = sopra_completion::cache::clear(command);
        self.completion = self.config.completion.then(CompletionEngine::new).flatten();
        result.map(|report| format!("{report}Worker completion memory reset.\n"))
    }

    fn refresh(&mut self) -> io::Result<()> {
        let config = Config::read_from(&self.config_path)?;
        match &mut self.widgets {
            Some(widgets) => {
                widgets.reconfigure(&config.left, &config.right, &self.cwd, &config.widgets)?
            }
            None => {
                self.widgets = Some(Widgets::configured(
                    config.left.clone(),
                    config.right.clone(),
                    self.cwd.clone(),
                    &config.widgets,
                )?)
            }
        }
        if config.completion != self.config.completion {
            self.completion = config.completion.then(CompletionEngine::new).flatten();
        }
        self.config = config;
        Ok(())
    }
}
