use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io::{self, Read},
    os::unix::ffi::OsStrExt,
    path::Path,
};

pub(super) struct Request {
    pub(super) arguments: Vec<OsString>,
    pub(super) environment: BTreeMap<OsString, OsString>,
}

pub(super) fn read(path: &Path) -> io::Result<Request> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(invalid("session request exceeds 16 MiB"));
    }
    parse(&bytes)
}

fn parse(bytes: &[u8]) -> io::Result<Request> {
    let mut fields = bytes.split(|byte| *byte == 0);
    if fields.next() != Some(b"S1") {
        return Err(invalid("invalid session header"));
    }
    let mut arguments = Vec::new();
    // The argument count preserves empty command buffers and empty prompts.
    let count = fields
        .next()
        .and_then(|value| std::str::from_utf8(value).ok())
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| invalid("invalid argument count"))?;
    for _ in 0..count {
        arguments.push(
            OsStr::from_bytes(fields.next().ok_or_else(|| invalid("missing argument"))?).to_owned(),
        );
    }
    let mut environment = BTreeMap::new();
    for field in fields.filter(|field| !field.is_empty()) {
        let separator = field
            .iter()
            .position(|byte| *byte == b'=')
            .ok_or_else(|| invalid("invalid environment entry"))?;
        if separator == 0 {
            return Err(invalid("empty environment name"));
        }
        environment.insert(
            OsStr::from_bytes(&field[..separator]).to_owned(),
            OsStr::from_bytes(&field[separator + 1..]).to_owned(),
        );
    }
    Ok(Request {
        arguments,
        environment,
    })
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::parse;
    #[test]
    fn requests_preserve_empty_arguments_and_multiline_environment() -> std::io::Result<()> {
        let request = parse(
            b"S1\0"
                .iter()
                .copied()
                .chain(b"2\0--buffer\0\0NAME=one\ntwo\0".iter().copied())
                .collect::<Vec<_>>()
                .as_slice(),
        )?;
        assert_eq!(request.arguments, ["--buffer", ""]);
        assert_eq!(
            request.environment.get(std::ffi::OsStr::new("NAME")),
            Some(&"one\ntwo".into())
        );
        assert!(parse(b"S1\0bad\0").is_err());
        assert!(
            parse(
                b"S1\0"
                    .iter()
                    .copied()
                    .chain(b"1\0".iter().copied())
                    .collect::<Vec<_>>()
                    .as_slice()
            )
            .is_ok()
        );
        Ok(())
    }
}
