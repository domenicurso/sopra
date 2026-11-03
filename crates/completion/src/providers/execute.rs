use super::{
    cache::{self, Cache},
    config::{self, Provider, Source},
    query::Query,
};
use crate::parser::{control::Control, process};
use std::{fs, path::Path, process::Command};

pub(super) fn resolve(
    query: &Query,
    control: &Control,
    cache: &mut Cache,
) -> Result<Option<Vec<String>>, String> {
    resolve_with(query, &config::load()?, control, cache)
}

pub(super) fn resolve_with(
    query: &Query,
    providers: &[Provider],
    control: &Control,
    cache: &mut Cache,
) -> Result<Option<Vec<String>>, String> {
    let mut values = Vec::new();
    let mut matched = false;
    for provider in providers.iter().filter(|provider| query.matches(provider)) {
        let Some(root) = query.root(provider) else {
            continue;
        };
        matched = true;
        if control.stopped() {
            return Err("provider request cancelled".into());
        }
        let key = cache::key(provider, query, &root);
        let result = match cache.get(key, provider) {
            Some(values) => values,
            None => {
                let values = run(provider, query, &root, control)?;
                cache.put(key, values.clone());
                values
            }
        };
        values.extend(result);
    }
    values.sort();
    values.dedup();
    values.truncate(4096);
    Ok(matched.then_some(values))
}

fn run(
    provider: &Provider,
    query: &Query,
    root: &Path,
    control: &Control,
) -> Result<Vec<String>, String> {
    match &provider.source {
        Source::Exec(argv) => {
            let mut command = Command::new(&argv[0]);
            command
                .args(&argv[1..])
                .current_dir(&query.request.cwd)
                .env("NO_COLOR", "1")
                .env("PAGER", "cat");
            let budget = control.child(provider.timeout);
            let (output, success) = process::run_controlled(&mut command, &budget, false)
                .ok_or_else(|| {
                    format!(
                        "{}: probe failed, timed out, or exceeded the output limit",
                        provider.id
                    )
                })?;
            if !success {
                return Err(format!("{}: probe exited unsuccessfully", provider.id));
            }
            Ok(output
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.chars().any(char::is_control))
                .take(4096)
                .map(str::to_string)
                .collect())
        }
        Source::Json { file, pointer } => json_values(provider, root, file, pointer),
    }
}

fn json_values(
    provider: &Provider,
    root: &Path,
    file: &str,
    pointer: &str,
) -> Result<Vec<String>, String> {
    let path = root.join(file);
    if fs::metadata(&path)
        .map_err(|error| error.to_string())?
        .len()
        > 1_000_000
    {
        return Err(format!("{}: JSON source exceeds one megabyte", provider.id));
    }
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let Some(value) = value.pointer(pointer) else {
        return Ok(Vec::new());
    };
    Ok(value
        .as_object()
        .ok_or_else(|| format!("{}: pointer must select an object", provider.id))?
        .keys()
        .filter(|name| !name.is_empty() && !name.chars().any(char::is_control))
        .cloned()
        .collect())
}
