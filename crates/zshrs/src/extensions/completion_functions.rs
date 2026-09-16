use std::path::PathBuf;

/// Completion-only builds use the user's installed `fpath` and never unpack
/// zshrs' full function bundle into `$HOME`.
pub fn functions_dir() -> Option<PathBuf> {
    None
}

pub fn ensure_installed() -> Option<usize> {
    Some(0)
}
