use std::{io, os::unix::process::CommandExt, path::Path, process::Command};

pub(super) fn replace(socket: &Path, parent: i32) -> io::Error {
    let result = (|| {
        let executable = std::env::current_exe()?;
        std::fs::remove_file(socket)?;
        // Exec preserves the shell-tracked PID and process group while replacing
        // every runtime service and closing the old worker's CLOEXEC descriptors.
        Ok::<_, io::Error>(
            Command::new(executable)
                .arg("--serve")
                .arg(socket)
                .arg(parent.to_string())
                .exec(),
        )
    })();
    result.unwrap_or_else(|error| error)
}
