use std::{
    collections::BTreeMap,
    io::{self, BufRead, Write},
    path::PathBuf,
};

pub struct Context {
    pub cwd: PathBuf,
    pub options: BTreeMap<String, String>,
    pub environment: BTreeMap<String, String>,
}

pub enum Action {
    None,
    ChangeDirectory(PathBuf),
}

pub trait Widget {
    fn render(&mut self, context: &Context) -> String;
    fn interactive(&self) -> bool {
        false
    }
    fn active(&self) -> bool {
        false
    }
    fn key(&mut self, _key: &str, _context: &Context) -> Action {
        Action::None
    }
}

pub fn run(mut widget: impl Widget) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let (context, event) = context(&line)?;
        let action = if event.is_empty() {
            Action::None
        } else {
            widget.key(&event, &context)
        };
        let directory = match action {
            Action::None => "-".to_owned(),
            Action::ChangeDirectory(path) => encode(&path.to_string_lossy()),
        };
        writeln!(
            stdout,
            "W1\t{}\t{}\t{}\t{}",
            encode(&widget.render(&context)),
            u8::from(widget.active()),
            u8::from(widget.interactive()),
            directory
        )?;
        stdout.flush()?;
    }
    Ok(())
}

fn context(line: &str) -> io::Result<(Context, String)> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() != 5 || fields[0] != "W1" {
        return Err(invalid());
    }
    Ok((
        Context {
            cwd: PathBuf::from(decode(fields[1])?),
            options: pairs(&decode(fields[2])?)?,
            environment: pairs(&decode(fields[3])?)?,
        },
        decode(fields[4])?,
    ))
}

fn pairs(text: &str) -> io::Result<BTreeMap<String, String>> {
    let mut fields = text.split_terminator('\0');
    let mut output = BTreeMap::new();
    while let Some(name) = fields.next() {
        output.insert(
            name.to_owned(),
            fields.next().ok_or_else(invalid)?.to_owned(),
        );
    }
    Ok(output)
}

pub(super) fn encode(text: &str) -> String {
    text.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn decode(text: &str) -> io::Result<String> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(invalid());
    }
    let bytes = (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).map_err(|_| invalid()))
        .collect::<io::Result<Vec<_>>>()?;
    String::from_utf8(bytes).map_err(|_| invalid())
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid widget message")
}
