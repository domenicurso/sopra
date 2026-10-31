use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let directory =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("src/prompt/widgets/builtins");
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut files = fs::read_dir(&directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    files.sort();
    let mut modules = String::new();
    let mut factories = Vec::new();
    for path in files
        .iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
    {
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or("invalid widget filename")?;
        modules.push_str(&format!("#[path = {path:?}] mod {name};\n"));
        factories.push(format!("({name:?}, {name}::create)"));
    }
    modules.push_str(&format!(
        "type Factory = fn(&WidgetContext<'_>) -> Box<dyn Widget>;\n\
         pub(super) const REGISTRY: &[(&str, Factory)] = &[{}];\n",
        factories.join(",")
    ));
    fs::write(
        PathBuf::from(env::var("OUT_DIR")?).join("widgets.rs"),
        modules,
    )?;
    Ok(())
}
