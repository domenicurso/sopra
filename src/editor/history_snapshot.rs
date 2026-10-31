use std::{
    collections::{HashMap, HashSet},
    io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "history_file.rs"]
mod file;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) command: String,
    pub(super) started: Option<u64>,
}

pub(super) fn read(path: &Path) -> io::Result<Vec<Entry>> {
    parse(&std::fs::read(path)?)
}

fn parse(bytes: &[u8]) -> io::Result<Vec<Entry>> {
    let mut fields = bytes.split(|byte| *byte == 0);
    let file_times = match fields.next() {
        Some(b"H3") => file::read(fields.next().unwrap_or_default())?,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid history snapshot header",
            ));
        }
    };
    let times = timestamps(fields.next().unwrap_or_default());
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    while let Some(event) = fields.next().filter(|field| !field.is_empty()) {
        let command = fields.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "history snapshot is missing a command",
            )
        })?;
        let Some(command) = valid_command(command) else {
            continue;
        };
        if seen.insert(command.to_string()) {
            entries.push(Entry {
                command: command.to_string(),
                started: std::str::from_utf8(event)
                    .ok()
                    .and_then(|id| times.get(id).copied())
                    .or_else(|| file_times.get(command).copied()),
            });
        }
    }
    Ok(entries)
}

fn valid_command(bytes: &[u8]) -> Option<&str> {
    // The editor cannot faithfully insert invalid UTF-8 or terminal controls.
    std::str::from_utf8(bytes).ok().filter(|text| {
        !text.trim().is_empty() && !text.chars().any(|c| c.is_control() && c != '\n')
    })
}

fn timestamps(bytes: &[u8]) -> HashMap<&str, u64> {
    // Zsh's listing escapes newlines in commands. Only its event and time columns
    // are used here; the separately framed commands preserve the original text.
    std::str::from_utf8(bytes)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut columns = line.split_whitespace();
            let event = columns.next()?;
            let time = columns
                .next()?
                .parse::<u64>()
                .ok()
                .filter(|time| *time > 0)?;
            Some((event, time))
        })
        .collect()
}

pub(super) fn description(entry: &Entry) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    age(entry.started, now).unwrap_or_else(|| "History".to_string())
}

fn age(started: Option<u64>, now: u64) -> Option<String> {
    let elapsed = now.checked_sub(started?)?;
    let (count, unit) = match elapsed {
        0..60 => (elapsed, "s"),
        60..3600 => (elapsed / 60, "m"),
        3600..86400 => (elapsed / 3600, "h"),
        86400..2592000 => (elapsed / 86400, "d"),
        2592000..31536000 => (elapsed / 2592000, "mo"),
        _ => (elapsed / 31536000, "y"),
    };
    Some(format!("{count}{unit} ago"))
}

#[cfg(test)]
mod tests {
    use super::{age, parse};

    #[test]
    fn snapshot_preserves_exact_commands_and_the_newest_duplicate_timestamp() -> std::io::Result<()>
    {
        let entries = parse(
            b"H3\0\0 1 10 echo old\n 2 20 echo recent\n 3 30 echo recent\n\0\
            3\0echo recent\0\
            2\0echo recent\0\
            1\0echo old\necho next\0\
            0\0\xff\0",
        )?;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].started, Some(30));
        assert_eq!(entries[1].command, "echo old\necho next");
        Ok(())
    }

    #[test]
    fn timestamps_are_optional_and_future_times_are_not_reported_as_recent() {
        assert_eq!(age(None, 3600), None);
        assert_eq!(age(Some(3601), 3600), None);
        assert_eq!(age(Some(3599), 3600).as_deref(), Some("1s ago"));
        assert_eq!(age(Some(3600), 3600).as_deref(), Some("0s ago"));
        assert_eq!(age(Some(3541), 3600).as_deref(), Some("59s ago"));
        assert_eq!(age(Some(3540), 3600).as_deref(), Some("1m ago"));
        assert_eq!(age(Some(1), 301).as_deref(), Some("5m ago"));
        assert_eq!(age(Some(1), 7201).as_deref(), Some("2h ago"));
        assert_eq!(age(Some(1), 172801).as_deref(), Some("2d ago"));
        assert!(parse(b"broken\0").is_err());
        assert!(parse(b"H3\0\0\0id").is_err());
    }
}
