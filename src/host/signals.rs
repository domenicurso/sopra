use std::{
    io,
    mem::MaybeUninit,
    sync::atomic::{AtomicI32, Ordering},
};

static RECEIVED: AtomicI32 = AtomicI32::new(0);
pub(super) struct Signals(Vec<(i32, libc::sigaction)>);
impl Signals {
    pub(super) fn install() -> io::Result<Self> {
        RECEIVED.store(0, Ordering::Relaxed);
        let mut signals = Self(Vec::new());
        for signal in [libc::SIGTERM, libc::SIGHUP, libc::SIGINT] {
            // sigaction is a C POD initialized before any field is read.
            let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
            action.sa_sigaction = record as *const () as usize;
            // Both pointers refer to stack-owned storage for the duration of the call.
            unsafe { libc::sigemptyset(&mut action.sa_mask) };
            let mut previous = MaybeUninit::uninit();
            if unsafe { libc::sigaction(signal, &action, previous.as_mut_ptr()) } == -1 {
                return Err(io::Error::last_os_error());
            }
            // Successful sigaction initialized previous; Drop restores installed handlers on error too.
            signals.0.push((signal, unsafe { previous.assume_init() }));
        }
        Ok(signals)
    }
    pub(super) fn received() -> Option<i32> {
        let signal = RECEIVED.load(Ordering::Relaxed);
        (signal != 0).then_some(signal)
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        for (signal, previous) in &self.0 {
            // Previous handlers were obtained from successful sigaction calls in install.
            unsafe { libc::sigaction(*signal, previous, std::ptr::null_mut()) };
        }
    }
}
extern "C" fn record(signal: i32) {
    RECEIVED.store(signal, Ordering::Relaxed);
}
