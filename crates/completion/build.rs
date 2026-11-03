use std::{
    collections::hash_map::DefaultHasher,
    env, fs,
    hash::{Hash, Hasher},
    path::Path,
};

fn main() -> std::io::Result<()> {
    let mut hash = DefaultHasher::new();
    fingerprint(Path::new("src"), &mut hash)?;
    fs::read("Cargo.toml")?.hash(&mut hash);
    if let Ok(lockfile) = fs::read("../../Cargo.lock") {
        lockfile.hash(&mut hash);
    }
    println!("cargo:rerun-if-changed=../../Cargo.lock");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    let output = env::var_os("OUT_DIR")
        .ok_or_else(|| std::io::Error::other("missing build output directory"))?;
    fs::write(
        Path::new(&output).join("graph-fingerprint"),
        format!("{:016x}", hash.finish()),
    )
}

fn fingerprint(directory: &Path, hash: &mut DefaultHasher) -> std::io::Result<()> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    for path in paths {
        if path.is_dir() {
            fingerprint(&path, hash)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "rs" || extension == "toml")
        {
            path.hash(hash);
            fs::read(path)?.hash(hash);
        }
    }
    Ok(())
}
