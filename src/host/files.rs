use std::path::{Path, PathBuf};

pub(super) fn at(text: &str, byte: usize, cwd: &Path) -> Option<PathBuf> {
    let prefix = text.get(..byte.min(text.len()))?;
    let quote = ['\"', '\''].into_iter().find_map(|quote| {
        let start = prefix.rfind(quote)?;
        let end = text.get(start + 1..)?.find(quote)? + start + 1;
        (byte <= end).then_some(start + 1..end)
    });
    let left = prefix
        .rfind(char::is_whitespace)
        .map_or(0, |index| index + 1);
    let right = text
        .get(byte.min(text.len())..)?
        .find(char::is_whitespace)
        .map_or(text.len(), |index| byte + index);
    let token = text
        .get(quote.unwrap_or(left..right))?
        .trim_matches(|ch: char| "\"'()[]{}<>,;".contains(ch));
    let path = resolve(token, cwd)?;
    if path.is_file() {
        return Some(path);
    }
    let mut token = token;
    while let Some((path, suffix)) = token.rsplit_once(':') {
        if suffix.parse::<usize>().is_err() {
            break;
        }
        token = path;
    }
    let path = resolve(token, cwd)?;
    path.is_file().then_some(path)
}
fn resolve(token: &str, cwd: &Path) -> Option<PathBuf> {
    Some(if let Some(tail) = token.strip_prefix("~/") {
        PathBuf::from(std::env::var_os("HOME")?).join(tail)
    } else {
        cwd.join(token.strip_prefix("file://").unwrap_or(token))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    #[test]
    fn relative_paths_and_line_suffixes_are_resolved_against_the_command_directory()
    -> io::Result<()> {
        let directory = tempfile::tempdir()?;
        std::fs::write(directory.path().join("file.rs"), "test")?;
        assert_eq!(
            at("error file.rs:12:3", 9, directory.path()),
            Some(directory.path().join("file.rs"))
        );
        std::fs::write(directory.path().join("with space.rs"), "test")?;
        assert_eq!(
            at("error \"with space.rs\"", 12, directory.path()),
            Some(directory.path().join("with space.rs"))
        );
        assert_eq!(at("not-a-file", 3, directory.path()), None);
        assert_eq!(at("é", 1, directory.path()), None);
        Ok(())
    }
}
