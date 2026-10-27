use std::sync::atomic::{AtomicUsize, Ordering};

use std::{io, mem::MaybeUninit};

static RESIZE_EPOCH: AtomicUsize = AtomicUsize::new(0);

pub(super) struct ResizeSignal {
    previous: libc::sigaction,
}

impl ResizeSignal {
    pub(super) fn install() -> io::Result<Self> {
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = record_resize as *const () as usize;
        // Let select return on SIGWINCH so the event loop notices resizes without waiting a frame.
        action.sa_flags = 0;
        // The signal action is stack-owned and remains valid for this system call.
        unsafe { libc::sigemptyset(&mut action.sa_mask) };
        let mut previous = MaybeUninit::uninit();
        // sigaction writes the previous action before returning success.
        if unsafe { libc::sigaction(libc::SIGWINCH, &action, previous.as_mut_ptr()) } == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful sigaction initialized the previous action.
        let previous = unsafe { previous.assume_init() };
        Ok(Self { previous })
    }

    pub(super) fn epoch() -> usize {
        RESIZE_EPOCH.load(Ordering::Relaxed)
    }
}

impl Drop for ResizeSignal {
    fn drop(&mut self) {
        // Restore the process handler before the shell resumes after the editor exits.
        unsafe { libc::sigaction(libc::SIGWINCH, &self.previous, std::ptr::null_mut()) };
    }
}

extern "C" fn record_resize(_: libc::c_int) {
    RESIZE_EPOCH.fetch_add(1, Ordering::Relaxed);
}
