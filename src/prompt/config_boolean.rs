use super::invalid;
use std::io;
use toml_edit::DocumentMut;

pub(super) fn read_bool(doc: &DocumentMut, keys: &[&str]) -> io::Result<Option<bool>> {
    let Some(item) = doc.get(keys[0]) else {
        return Ok(None);
    };
    let table = item.as_table_like().ok_or_else(|| invalid(keys))?;
    let Some(item) = table.get(keys[1]) else {
        return Ok(None);
    };
    item.as_bool().map(Some).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} must be a boolean", keys.join(".")),
        )
    })
}
