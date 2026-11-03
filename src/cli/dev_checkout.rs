use std::{
    env, fs, io,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

pub(super) fn current() -> io::Result<PathBuf> {
    match fs::read(link_path()?) {
        Ok(bytes) => return validate(&PathBuf::from(std::ffi::OsString::from_vec(bytes))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    if let Some(path) = env::var_os("SOPRA_REPO_ROOT") {
        return validate(Path::new(&path));
    }
    let cwd = env::current_dir()?;
    let executable = env::current_exe()?;
    [cwd.as_path(), executable.as_path()]
        .into_iter()
        .flat_map(Path::ancestors)
        .find(|path| is_checkout(path))
        .map(fs::canonicalize)
        .unwrap_or_else(|| Err(io::Error::other(
            "no Sopra checkout found; run `sopra dev link /path/to/sopra` or set SOPRA_REPO_ROOT",
        )))
}

pub(super) fn link(path: &Path) -> io::Result<PathBuf> {
    let checkout = validate(path)?;
    let destination = link_path()?;
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::other("invalid checkout link path"))?;
    fs::create_dir_all(parent)?;
    fs::write(destination, checkout.as_os_str().as_bytes())?;
    Ok(checkout)
}

pub(super) fn unlink() -> io::Result<()> {
    match fs::remove_file(link_path()?) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn link_path() -> io::Result<PathBuf> {
    Ok(crate::config::path()?.with_file_name("dev-checkout"))
}

fn validate(path: &Path) -> io::Result<PathBuf> {
    if !is_checkout(path) {
        return Err(io::Error::other(format!(
            "{} is not a Sopra source checkout; update `sopra dev link` or SOPRA_REPO_ROOT",
            path.display(),
        )));
    }
    fs::canonicalize(path)
}

fn is_checkout(path: &Path) -> bool {
    [
        "Cargo.toml",
        "scripts/build.sh",
        "scripts/start.sh",
        "zsh/editor.zsh",
    ]
    .iter()
    .all(|file| path.join(file).is_file())
}
