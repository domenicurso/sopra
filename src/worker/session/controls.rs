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
            Command::ConfigShow => Ok(self.config.effective()),
            Command::WidgetsList => inspection::list(&self.config.widgets, self.widgets.as_ref()),
            Command::Ping => Err(io::Error::other("health requests belong to the listener")),
        }
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
