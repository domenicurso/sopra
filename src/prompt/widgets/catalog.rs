use super::{builtins, settings::Settings};
use std::{io, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Definition {
    pub(crate) name: String,
    pub(crate) source: Option<PathBuf>,
}

pub(crate) fn read(settings: &Settings) -> io::Result<Vec<Definition>> {
    let mut entries = builtins::REGISTRY
        .iter()
        .map(|(name, _)| Definition {
            name: (*name).into(),
            source: None,
        })
        .collect::<Vec<_>>();
    if let Some(root) = &settings.root {
        let files = match std::fs::read_dir(root.join("widgets")) {
            Ok(files) => Some(files),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        for file in files.into_iter().flatten() {
            let path = file?.path();
            let Some(name) = name(&path) else { continue };
            if !entries.iter().any(|entry| entry.name == name) {
                entries.push(Definition {
                    name,
                    source: Some(path),
                });
            }
        }
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

fn name(path: &std::path::Path) -> Option<String> {
    let name = path.file_stem()?.to_str()?;
    (path.extension()? == "rs"
        && path.is_file()
        && !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
    .then(|| name.into())
}
