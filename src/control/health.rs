use std::{
    io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use toml_edit::{DocumentMut, value};

pub(crate) struct Health {
    pub(crate) config: PathBuf,
    pub(crate) pid: i64,
    pub(crate) shell: i64,
    pub(crate) generation: String,
    pub(crate) busy: bool,
}
impl Health {
    pub(crate) fn new(config: PathBuf, shell: i32) -> io::Result<Self> {
        Ok(Self {
            config,
            shell: shell.into(),
            pid: std::process::id().into(),
            generation: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_nanos()
                .to_string(),
            busy: false,
        })
    }
    pub(crate) fn encode(&self, busy: bool) -> String {
        let mut document = DocumentMut::new();
        document["config"] = value(self.config.to_string_lossy().as_ref());
        document["pid"] = value(self.pid);
        document["shell"] = value(self.shell);
        document["generation"] = value(&self.generation);
        document["busy"] = value(busy);
        document.to_string()
    }
    pub(crate) fn parse(source: &str) -> io::Result<Self> {
        let document = source.parse::<DocumentMut>().map_err(io::Error::other)?;
        let invalid = || io::Error::other("invalid worker health response");
        Ok(Self {
            config: document
                .get("config")
                .and_then(|v| v.as_str())
                .ok_or_else(invalid)?
                .into(),
            generation: document
                .get("generation")
                .and_then(|v| v.as_str())
                .ok_or_else(invalid)?
                .into(),
            pid: document
                .get("pid")
                .and_then(|v| v.as_integer())
                .filter(|v| *v > 1)
                .ok_or_else(invalid)?,
            shell: document
                .get("shell")
                .and_then(|v| v.as_integer())
                .filter(|v| *v > 1)
                .ok_or_else(invalid)?,
            busy: document
                .get("busy")
                .and_then(|v| v.as_bool())
                .ok_or_else(invalid)?,
        })
    }
}
