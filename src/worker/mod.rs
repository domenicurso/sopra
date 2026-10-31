mod request;
mod session;

use std::{
    error::Error,
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    thread,
    time::Duration,
};

use crate::{arguments::Args, palette::TerminalPalette, protocol};

pub(crate) fn once() -> Result<(), Box<dyn Error>> {
    let args = Args::parse().map_err(|message| format!("sopra: {message}"))?;
    let result = session::run(args, &mut None)?;
    protocol::write_result(&result, &mut std::io::stdout().lock())?;
    Ok(())
}

pub(crate) fn serve(socket: &Path, parent: i32) -> Result<(), Box<dyn Error>> {
    if parent <= 1 {
        return Err("invalid shell PID".into());
    }
    // The editor must read the foreground shell's terminal without SIGTTIN.
    let group = unsafe { libc::getpgid(parent) };
    if group < 0 || unsafe { libc::setpgid(0, group) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let listener = UnixListener::bind(socket)?;
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            // kill with signal 0 only checks whether the owning shell still exists.
            if unsafe { libc::kill(parent, 0) } != 0 {
                std::process::exit(0);
            }
        }
    });
    let mut palette = None;
    for connection in listener.incoming() {
        let mut connection = connection?;
        if let Err(error) = edit(&mut connection, &mut palette) {
            writeln!(
                connection,
                "E1\t{}",
                error.to_string().replace(['\n', '\r'], " ")
            )?;
        }
        connection.flush()?;
    }
    Ok(())
}

fn edit(
    connection: &mut UnixStream,
    palette: &mut Option<TerminalPalette>,
) -> Result<(), Box<dyn Error>> {
    let mut path = String::new();
    use std::io::Read;
    BufReader::new(&mut *connection)
        .take(4097)
        .read_line(&mut path)?;
    if path.len() > 4096 || !path.ends_with('\n') {
        return Err("invalid session request path".into());
    }
    let request = request::read(Path::new(path.trim_end_matches('\n')))?;
    sopra_shell_context::replace(request.environment);
    let args = Args::from_args(request.arguments.into_iter())?;
    let result = session::run(args, palette)?;
    protocol::write_result(&result, connection)?;
    Ok(())
}
