use std::{collections::HashMap, ffi::OsStr, io, os::unix::ffi::OsStrExt, path::Path};

pub(super) fn read(path: &[u8]) -> io::Result<HashMap<String, u64>> {
    match std::fs::read(Path::new(OsStr::from_bytes(path))) {
        Ok(bytes) => Ok(parse(&bytes)),
        // The file can disappear between the shell's check and worker startup.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(HashMap::new()),
        Err(error) => Err(error),
    }
}

fn parse(bytes: &[u8]) -> HashMap<String, u64> {
    let mut times = HashMap::new();
    let mut command = Vec::new();
    let mut started = None;
    for line in bytes.split(|byte| *byte == b'\n') {
        let line = if command.is_empty() {
            let Some((time, text)) = extended_entry(line) else {
                continue;
            };
            started = Some(time);
            text
        } else {
            line
        };
        if let Some(text) = line.strip_suffix(b"\\") {
            command.extend_from_slice(text);
            command.push(b'\n');
            continue;
        }
        command.extend_from_slice(line);
        if let (Some(time), Ok(text)) = (started, String::from_utf8(unmetafy(&command))) {
            times.insert(text, time);
        }
        command.clear();
    }
    times
}

fn extended_entry(line: &[u8]) -> Option<(u64, &[u8])> {
    let line = line.strip_prefix(b": ")?;
    let separator = line.iter().position(|byte| *byte == b';')?;
    let header = std::str::from_utf8(&line[..separator]).ok()?;
    let (time, duration) = header.split_once(':')?;
    duration.parse::<u64>().ok()?;
    let time = time.parse::<u64>().ok().filter(|time| *time > 0)?;
    Some((time, &line[separator + 1..]))
}

fn unmetafy(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len());
    let mut bytes = bytes.iter().copied();
    while let Some(byte) = bytes.next() {
        // Zsh escapes its internal Meta byte and control bytes with Meta + XOR 32.
        output.push(if byte == 0x83 {
            bytes.next().map_or(byte, |next| next ^ 32)
        } else {
            byte
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn saved_timestamps_preserve_multiline_commands_and_newest_duplicates() {
        let times = parse(b"legacy command\n: 10:0;echo hi\n: 20:1;echo hi\n: 30:0;echo one\\\necho two\n: 0:0;unknown\n: broken;invalid\n");
        assert_eq!(times.get("echo hi"), Some(&20));
        assert_eq!(times.get("echo one\necho two"), Some(&30));
        assert_eq!(times.len(), 2);
    }

    #[test]
    fn saved_history_decodes_zsh_meta_bytes_and_ignores_invalid_utf8() {
        let times = parse(b": 10:0;echo \xc3\x83\xa3\n: 20:0;\xff\n");
        assert_eq!(times.get("echo \u{c3}"), Some(&10));
        assert_eq!(times.len(), 1);
    }
}
