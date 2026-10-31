//! Shell snapshots are shared explicitly; the worker never mutates process env.
use std::{collections::BTreeMap, env::VarError, ffi::OsString, sync::RwLock};

static SHELL: RwLock<Option<BTreeMap<OsString, OsString>>> = RwLock::new(None);

pub fn replace(values: BTreeMap<OsString, OsString>) {
    *SHELL.write().unwrap_or_else(|error| error.into_inner()) = Some(values);
}

pub fn vars_os() -> BTreeMap<OsString, OsString> {
    SHELL
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .unwrap_or_else(|| std::env::vars_os().collect())
}

pub fn var_os(name: impl AsRef<std::ffi::OsStr>) -> Option<OsString> {
    let shell = SHELL.read().unwrap_or_else(|error| error.into_inner());
    match shell.as_ref() {
        Some(values) => values.get(name.as_ref()).cloned(),
        None => std::env::var_os(name),
    }
}

pub fn var(name: impl AsRef<std::ffi::OsStr>) -> Result<String, VarError> {
    var_os(name)
        .ok_or(VarError::NotPresent)?
        .into_string()
        .map_err(VarError::NotUnicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_replace_removed_values_without_mutating_process_environment() {
        let original = std::env::var_os("SOPRA_SNAPSHOT_TEST");
        replace(BTreeMap::from([(
            "SOPRA_SNAPSHOT_TEST".into(),
            "fresh".into(),
        )]));
        assert_eq!(var("SOPRA_SNAPSHOT_TEST").ok().as_deref(), Some("fresh"));
        assert_eq!(std::env::var_os("SOPRA_SNAPSHOT_TEST"), original);
        replace(BTreeMap::new());
        assert!(var_os("SOPRA_SNAPSHOT_TEST").is_none());
        assert_eq!(std::env::var_os("SOPRA_SNAPSHOT_TEST"), original);
    }
}
