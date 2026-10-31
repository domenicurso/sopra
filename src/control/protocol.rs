use std::io::{self, BufRead, Read, Write};

const MAX_MESSAGE: u64 = 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Ping,
    Refresh,
    Restart,
    ConfigShow,
    WidgetsList,
}
impl Command {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::Refresh => "refresh",
            Self::Restart => "restart",
            Self::ConfigShow => "config-show",
            Self::WidgetsList => "widgets-list",
        }
    }
}

pub(crate) fn read_request(line: &str) -> io::Result<Option<Command>> {
    let Some(name) = line.strip_prefix("C1\t") else {
        return Ok(None);
    };
    Ok(Some(match name.trim_end_matches('\n') {
        "ping" => Command::Ping,
        "refresh" => Command::Refresh,
        "restart" => Command::Restart,
        "config-show" => Command::ConfigShow,
        "widgets-list" => Command::WidgetsList,
        _ => return Err(io::Error::other("unknown worker control request")),
    }))
}

pub(crate) fn write_response(
    output: &mut impl Write,
    response: io::Result<String>,
) -> io::Result<()> {
    let (status, body) = match response {
        Ok(body) => ("ok", body),
        Err(error) => ("error", error.to_string()),
    };
    writeln!(output, "C1\t{status}\t{}", body.len())?;
    output.write_all(body.as_bytes())?;
    output.flush()
}

pub(crate) fn read_response(input: &mut impl BufRead) -> io::Result<String> {
    let mut header = String::new();
    (&mut *input).take(256).read_line(&mut header)?;
    let fields = header
        .trim_end_matches('\n')
        .split('\t')
        .collect::<Vec<_>>();
    if fields.len() != 3 || fields[0] != "C1" || !header.ends_with('\n') {
        return Err(io::Error::other("invalid worker control response"));
    }
    let size = fields[2]
        .parse::<u64>()
        .ok()
        .filter(|size| *size <= MAX_MESSAGE)
        .ok_or_else(|| io::Error::other("invalid worker response length"))?;
    let mut body = vec![0; size as usize];
    input.read_exact(&mut body)?;
    let body = String::from_utf8(body).map_err(io::Error::other)?;
    match fields[1] {
        "ok" => Ok(body),
        "error" => Err(io::Error::other(body)),
        _ => Err(io::Error::other("invalid worker response status")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn responses_preserve_multiline_text_and_propagate_failures() -> io::Result<()> {
        let mut bytes = Vec::new();
        write_response(&mut bytes, Ok("line\n世界\tvalue".into()))?;
        assert_eq!(read_response(&mut bytes.as_slice())?, "line\n世界\tvalue");
        bytes.clear();
        write_response(&mut bytes, Err(io::Error::other("bad config")))?;
        assert_eq!(
            read_response(&mut bytes.as_slice())
                .unwrap_err()
                .to_string(),
            "bad config"
        );
        for invalid in ["C1\tok\t1048577\n", "C1\tok\t3\nx", "C1\tmaybe\t0\n"] {
            assert!(read_response(&mut invalid.as_bytes()).is_err());
        }
        assert!(read_request("C1\tunknown\n").is_err());
        Ok(())
    }
}
