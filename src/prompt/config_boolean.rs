use super::invalid;
use std::io;
use toml_edit::{DocumentMut, Item, value};

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

pub(super) fn set_bool(doc: &mut DocumentMut, key: &str, flag: Option<bool>) -> io::Result<()> {
    let Some(flag) = flag else { return Ok(()) };
    if !doc.as_table().contains_key("editor") {
        doc["editor"] = Item::Table(toml_edit::Table::new());
    }
    let table = doc["editor"]
        .as_table_mut()
        .ok_or_else(|| invalid(&["editor"]))?;
    if let Some(existing) = table.get_mut(key) {
        let previous = existing
            .as_value()
            .ok_or_else(|| invalid(&["editor", key]))?;
        let mut replacement = value(flag);
        if let Some(next) = replacement.as_value_mut() {
            *next.decor_mut() = previous.decor().clone();
        }
        *existing = replacement;
    } else {
        table.insert(key, value(flag));
    }
    Ok(())
}
