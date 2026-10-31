use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{Receiver, TryRecvError},
};
fn open(path: &Path) -> io::Result<Receiver<io::Result<()>>> {
    let mut command = Command::new(if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    });
    let mut child = command
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = child.wait().and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "file opener exited with {status}"
                )))
            }
        });
        let _ = sender.send(result);
    });
    Ok(receiver)
}

#[derive(Default)]
pub(super) struct Openers(Vec<(PathBuf, Receiver<io::Result<()>>)>);
impl Openers {
    pub(super) fn start(&mut self, path: PathBuf) -> io::Result<()> {
        if self.0.len() >= 16 {
            return Err(io::Error::other("too many pending file open requests"));
        }
        self.0.push((path.clone(), open(&path)?));
        Ok(())
    }
    pub(super) fn poll(&mut self) -> Vec<String> {
        let mut notices = Vec::new();
        self.0.retain(|(path, receiver)| match receiver.try_recv() {
            Err(TryRecvError::Empty) => true,
            result => {
                let result =
                    result.unwrap_or_else(|_| Err(io::Error::other("file opener stopped")));
                notices.push(match result {
                    Ok(()) => format!("Opened {}", path.display()),
                    Err(error) => format!("Could not open {}: {error}", path.display()),
                });
                false
            }
        });
        notices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn asynchronous_opener_failures_keep_the_path_and_original_cause() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        // The receiver stays alive until poll consumes this deliberately failed request.
        sender
            .send(Err(io::Error::other("permission denied")))
            .expect("receiver is retained");
        let mut openers = Openers(vec![("/tmp/example".into(), receiver)]);
        assert_eq!(
            openers.poll(),
            ["Could not open /tmp/example: permission denied"]
        );
        assert!(openers.poll().is_empty());
    }
}
