use std::{fs, path::PathBuf, time::Duration};
use toml_edit::{DocumentMut, Table};

#[derive(Clone, Debug)]
pub(super) struct Provider {
    pub(super) id: String,
    pub(super) command: String,
    pub(super) path: Vec<String>,
    pub(super) slot: Slot,
    pub(super) source: Source,
    pub(super) marker: Option<String>,
    pub(super) watch: Vec<String>,
    pub(super) ttl: Duration,
    pub(super) timeout: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Slot {
    Positional(usize),
    Option(String),
}
#[derive(Clone, Debug)]
pub(super) enum Source {
    Exec(Vec<String>),
    Json { file: String, pointer: String },
}

pub(super) fn load() -> Result<Vec<Provider>, String> {
    let mut providers = parse(include_str!("builtin.toml"))?;
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(text) => {
            for provider in parse(&text)? {
                providers.retain(|existing| existing.id != provider.id);
                providers.push(provider);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("{}: {error}", path.display())),
    }
    Ok(providers)
}

pub(super) fn config_path() -> PathBuf {
    sopra_shell_context::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            sopra_shell_context::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
        })
        .unwrap_or_default()
        .join("sopra/completions.toml")
}

pub(super) fn parse(text: &str) -> Result<Vec<Provider>, String> {
    let document = text
        .parse::<DocumentMut>()
        .map_err(|error| error.to_string())?;
    if document.iter().any(|(name, _)| name != "provider") {
        return Err("only [[provider]] tables are supported".into());
    }
    if document.get("provider").is_none() {
        return Ok(Vec::new());
    }
    let tables = document
        .get("provider")
        .and_then(|item| item.as_array_of_tables())
        .ok_or("expected [[provider]] tables")?;
    let mut providers = Vec::new();
    for table in tables {
        if let Some((name, _)) = table.iter().find(|(name, _)| {
            ![
                "id",
                "command",
                "path",
                "option",
                "positional",
                "exec",
                "json_file",
                "pointer",
                "root_marker",
                "watch",
                "ttl_ms",
                "timeout_ms",
                "enabled",
            ]
            .contains(name)
        }) {
            return Err(format!("unknown provider field: {name}"));
        }
        if table.get("enabled").and_then(|item| item.as_bool()) == Some(false) {
            providers.push(disabled(table)?);
        } else {
            providers.push(provider(table)?);
        }
    }
    Ok(providers)
}

fn provider(table: &Table) -> Result<Provider, String> {
    let id = string(table, "id")?;
    let slot = match (
        table.get("positional").and_then(|item| item.as_integer()),
        table.get("option").and_then(|item| item.as_str()),
    ) {
        (Some(index), None) if (0..=128).contains(&index) => Slot::Positional(index as usize),
        (None, Some(name)) if name.starts_with('-') => Slot::Option(name.into()),
        _ => return Err(format!("{id}: specify one positional index or option")),
    };
    let source = match (table.get("exec"), table.get("json_file")) {
        (Some(_), None) => {
            let argv = array(table, "exec")?;
            if argv.is_empty() {
                return Err(format!("{id}: exec must not be empty"));
            }
            Source::Exec(argv)
        }
        (None, Some(_)) => Source::Json {
            file: string(table, "json_file")?,
            pointer: string(table, "pointer")?,
        },
        _ => return Err(format!("{id}: specify one exec or json_file source")),
    };
    Ok(Provider {
        id,
        command: string(table, "command")?,
        path: array(table, "path")?,
        slot,
        source,
        marker: table
            .get("root_marker")
            .and_then(|item| item.as_str())
            .map(str::to_string),
        watch: array(table, "watch")?,
        ttl: duration(table, "ttl_ms", 300, 5000)?,
        timeout: duration(table, "timeout_ms", 150, 500)?,
    })
}

fn disabled(table: &Table) -> Result<Provider, String> {
    Ok(Provider {
        id: string(table, "id")?,
        command: String::new(),
        path: Vec::new(),
        slot: Slot::Positional(0),
        source: Source::Exec(Vec::new()),
        marker: None,
        watch: Vec::new(),
        ttl: Duration::ZERO,
        timeout: Duration::ZERO,
    })
}

fn string(table: &Table, name: &str) -> Result<String, String> {
    table
        .get(name)
        .and_then(|item| item.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("provider requires a nonempty {name} string"))
}

fn array(table: &Table, name: &str) -> Result<Vec<String>, String> {
    let Some(item) = table.get(name) else {
        return Ok(Vec::new());
    };
    item.as_array()
        .ok_or_else(|| format!("{name} must be an array"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{name} must contain strings"))
        })
        .collect()
}

fn duration(table: &Table, name: &str, default: u64, maximum: u64) -> Result<Duration, String> {
    let value = match table.get(name) {
        Some(item) => item
            .as_integer()
            .filter(|value| *value >= 0)
            .map(|value| value as u64)
            .ok_or_else(|| format!("{name} must be a nonnegative integer"))?,
        None => default,
    };
    if value > maximum {
        return Err(format!("{name} must be at most {maximum}"));
    }
    Ok(Duration::from_millis(value))
}
