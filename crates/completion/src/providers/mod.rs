mod cache;
mod config;
mod execute;
mod query;
mod worker;

use crate::{CompletionItem, CompletionKind, CompletionSource};
pub(crate) use query::{Query, plan};
pub(crate) use worker::Worker;

fn items(query: &Query, values: Vec<String>) -> Vec<CompletionItem> {
    let needle = brush_parser::unquote_str(&query.query);
    values
        .into_iter()
        .filter(|value| crate::engine::values::matches_query(value, &needle))
        .map(|value| {
            let mut item = CompletionItem::with_range(
                &value,
                crate::literal::quote(&value, &query.query),
                Some("Live completion value".into()),
                CompletionKind::Value,
                query.replace.clone(),
                CompletionSource::ShellContext,
            );
            item.group = Some("Live values".into());
            item
        })
        .collect()
}

pub(crate) fn inspect() -> Result<String, String> {
    let providers = config::load()?;
    Ok(format!(
        "Configuration: {}\nProviders:\n{}",
        config::config_path().display(),
        providers
            .iter()
            .filter(|provider| !provider.command.is_empty())
            .map(|provider| format!(
                "  {}: {} {:?} {:?}; timeout={}ms ttl={}ms; {:?}\n",
                provider.id,
                provider.command,
                provider.path,
                provider.slot,
                provider.timeout.as_millis(),
                provider.ttl.as_millis(),
                provider.source
            ))
            .collect::<String>()
    ))
}

#[cfg(test)]
mod tests;

pub(crate) fn complete(
    query: &Query,
    control: &crate::parser::control::Control,
) -> Result<Option<Vec<CompletionItem>>, String> {
    let mut cache = cache::Cache::new();
    execute::resolve(query, control, &mut cache)
        .map(|values| values.map(|values| items(query, values)))
}
