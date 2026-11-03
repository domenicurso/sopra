mod request;
mod restart;
mod server;
mod session;

use std::{error::Error, io::Write, os::unix::net::UnixStream, path::Path, thread, time::Duration};

use crate::{arguments::Args, protocol};

pub(crate) fn once() -> Result<(), Box<dyn Error>> {
    let args = Args::parse().map_err(|message| format!("sopra: {message}"))?;
    let mut session = session::Session::new()?;
    let styled_transcript = args.styled_transcript;
    let result = session.run(args)?;
    let prompts = session.shell_prompts()?;
    protocol::write_result(
        &result,
        &mut std::io::stdout().lock(),
        (&prompts.0, &prompts.1),
        styled_transcript,
    )?;
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
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            // kill with signal 0 only checks whether the owning shell still exists.
            if unsafe { libc::kill(parent, 0) } != 0 {
                std::process::exit(0);
            }
        }
    });
    let mut session = session::Session::new()?;
    let health = crate::control::Health::new(session.config_path().to_owned(), parent)?;
    let connections = server::listen(socket, health)?;
    for mut connection in connections {
        match dispatch(&mut connection, &mut session) {
            Ok(Next::Restart) => {
                drop(session);
                return Err(restart::replace(socket, parent).into());
            }
            Ok(Next::Continue) => {}
            Err(error) => eprintln!("sopra: {error}"),
        }
    }
    Ok(())
}

enum Next {
    Continue,
    Restart,
}

fn dispatch(
    connection: &mut server::Connection,
    session: &mut session::Session,
) -> Result<Next, Box<dyn Error>> {
    match &connection.operation {
        server::Operation::Edit(path) => {
            if let Err(error) = edit(&mut connection.stream, session, path) {
                writeln!(
                    connection.stream,
                    "E1\t{}",
                    error.to_string().replace(['\n', '\r'], " ")
                )?;
            }
            connection.stream.flush()?;
        }
        server::Operation::Control(command) => {
            let result = session.control(command.clone());
            let next = if *command == crate::control::Command::Restart && result.is_ok() {
                Next::Restart
            } else {
                Next::Continue
            };
            crate::control::write_response(&mut connection.stream, result)?;
            return Ok(next);
        }
    }
    Ok(Next::Continue)
}

fn edit(
    connection: &mut UnixStream,
    session: &mut session::Session,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    let request = request::read(path)?;
    sopra_shell_context::replace(request.environment);
    let args = Args::from_args(request.arguments.into_iter())?;
    let styled_transcript = args.styled_transcript;
    let result = session.run(args)?;
    let prompts = session.shell_prompts()?;
    protocol::write_result(
        &result,
        connection,
        (&prompts.0, &prompts.1),
        styled_transcript,
    )?;
    Ok(())
}
