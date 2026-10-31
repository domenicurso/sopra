use std::{
    io,
    path::{Path, PathBuf},
};

pub(super) struct Bootstrap {
    pub(super) directory: tempfile::TempDir,
}
impl Bootstrap {
    pub(super) fn create() -> io::Result<Self> {
        let directory = tempfile::Builder::new().prefix("sopra-shell-").tempdir()?;
        let executable = std::env::current_exe()?;
        let parent = executable
            .parent()
            .ok_or_else(|| io::Error::other("executable has no parent"))?;
        let bridge = [
            parent.join("../../zsh/editor.zsh"),
            parent.join("../share/sopra/editor.zsh"),
        ]
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| io::Error::other("cannot find Zsh bridge beside Sopra executable"))?
        .canonicalize()?;
        let original = std::env::var_os("ZDOTDIR")
            .or_else(|| std::env::var_os("HOME"))
            .ok_or_else(|| io::Error::other("HOME and ZDOTDIR are unset"))?;
        let original = PathBuf::from(original);
        let env = format!(
            "typeset -gx _SOPRA_HOST_SHELL_PID=$$\ntypeset -g ZDOTDIR={}\n[[ ! -r $ZDOTDIR/.zshenv ]] || source \"$ZDOTDIR/.zshenv\"\ntypeset -gx _SOPRA_USER_ZDOTDIR=$ZDOTDIR\nZDOTDIR={}\n",
            quote(&original),
            quote(directory.path())
        );
        std::fs::write(directory.path().join(".zshenv"), env)?;
        let rc = format!(
            "ZDOTDIR=$_SOPRA_USER_ZDOTDIR\nunset _SOPRA_USER_ZDOTDIR\n[[ ! -r $ZDOTDIR/.zshrc ]] || source \"$ZDOTDIR/.zshrc\"\nsource {}\n",
            quote(&bridge)
        );
        std::fs::write(directory.path().join(".zshrc"), rc)?;
        Ok(Self { directory })
    }
}
fn quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}
