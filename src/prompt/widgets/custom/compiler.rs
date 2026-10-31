use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

include!(concat!(env!("OUT_DIR"), "/widget_sdk.rs"));

static BUILDS: AtomicU64 = AtomicU64::new(0);

pub(super) fn build(source: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let mut hash = DefaultHasher::new();
    bytes.hash(&mut hash);
    SDK_SOURCES.hash(&mut hash);
    std::env::consts::OS.hash(&mut hash);
    std::env::consts::ARCH.hash(&mut hash);
    let directory = cache_root()?.join(format!("{:016x}", hash.finish()));
    fs::create_dir_all(&directory)?;
    let binary = directory.join("widget");
    if binary.is_file() {
        return Ok(binary);
    }
    let build = directory.join(format!(
        "build-{}-{}",
        std::process::id(),
        BUILDS.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&build)?;
    fs::create_dir_all(build.join("sdk"))?;
    for (name, contents) in SDK_SOURCES {
        fs::write(build.join("sdk").join(name), contents)?;
    }
    fs::write(build.join("widget.rs"), bytes)?;
    fs::write(
        build.join("main.rs"),
        "#[path = \"sdk/lib.rs\"] mod sopra;\ninclude!(\"widget.rs\");\n",
    )?;
    let errors = fs::File::create(build.join("errors.log"))?;
    let mut child = Command::new("rustc")
        .args(["--edition=2024", "-O", "--crate-name", "sopra_widget"])
        .arg(build.join("main.rs"))
        .arg("-o")
        .arg(build.join("widget"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(errors)
        .spawn()?;
    wait_for_compiler(&mut child, source, &build)?;
    fs::rename(build.join("widget"), &binary)?;
    fs::remove_dir_all(&build)?;
    Ok(binary)
}

fn wait_for_compiler(child: &mut Child, source: &Path, build: &Path) -> io::Result<()> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            return Err(io::Error::other(format!(
                "{} failed to compile; see {}",
                source.display(),
                build.join("errors.log").display()
            )));
        }
        if started.elapsed() > Duration::from_secs(30) {
            child.kill()?;
            child.wait()?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "widget compilation exceeded 30 seconds",
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn record_error(error: &io::Error) -> io::Result<()> {
    use std::io::Write;
    let root = cache_root()?;
    fs::create_dir_all(&root)?;
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("errors.log"))?;
    writeln!(log, "{error}")
}

fn cache_root() -> io::Result<PathBuf> {
    let env = sopra_shell_context::var_os;
    let root = env("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .ok_or_else(|| io::Error::other("no widget cache directory"))?;
    Ok(root.join("sopra/widgets"))
}
