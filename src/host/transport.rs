use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread,
};

pub(super) struct Writer {
    sender: SyncSender<Vec<u8>>,
    failures: Receiver<io::Error>,
    pending: VecDeque<Vec<u8>>,
    bytes: usize,
}
impl Writer {
    pub(super) fn new(mut writer: Box<dyn Write + Send>) -> Self {
        let (sender, input) = mpsc::sync_channel::<Vec<u8>>(4);
        let (errors, failures) = mpsc::sync_channel(1);
        thread::spawn(move || {
            for bytes in input {
                if let Err(error) = writer.write_all(&bytes).and_then(|()| writer.flush()) {
                    let _ = errors.send(error);
                    break;
                }
            }
        });
        Self {
            sender,
            failures,
            pending: VecDeque::new(),
            bytes: 0,
        }
    }
    pub(super) fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        if !bytes.is_empty() {
            self.bytes += bytes.len();
            self.pending.push_back(bytes.to_vec());
        }
        self.flush()
    }
    pub(super) fn flush(&mut self) -> io::Result<()> {
        if let Ok(error) = self.failures.try_recv() {
            return Err(error);
        }
        while let Some(bytes) = self.pending.pop_front() {
            let length = bytes.len();
            match self.sender.try_send(bytes) {
                Ok(()) => self.bytes -= length,
                Err(TrySendError::Full(bytes)) => {
                    self.pending.push_front(bytes);
                    break;
                }
                Err(TrySendError::Disconnected(_)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "shell input closed",
                    ));
                }
            }
        }
        Ok(())
    }
    pub(super) fn ready(&self) -> bool {
        self.bytes < 65536
    }
}

pub(super) fn reader(mut reader: Box<dyn Read + Send>) -> Receiver<io::Result<Vec<u8>>> {
    let (sender, output) = mpsc::sync_channel(16);
    thread::spawn(move || {
        loop {
            let mut bytes = vec![0; 65536];
            match reader.read(&mut bytes) {
                Ok(0) => break,
                Ok(length) => {
                    bytes.truncate(length);
                    if sender.send(Ok(bytes)).is_err() {
                        break;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
                Err(error) => {
                    let _ = sender.send(Err(error));
                    break;
                }
            }
        }
    });
    output
}
