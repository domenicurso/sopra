use super::ansi::move_to;

pub(super) fn clear_rows(output: &mut Vec<u8>, origin_row: u16, rows: &[u16]) {
    for &row in rows {
        move_to(output, 0, origin_row.saturating_add(row));
        output.extend_from_slice(b"\x1b[2K");
    }
}

pub(super) fn clear_viewport(output: &mut Vec<u8>, origin_row: u16) {
    move_to(output, 0, origin_row);
    output.extend_from_slice(b"\x1b[J");
}

#[cfg(test)]
mod tests {
    use super::clear_viewport;

    #[test]
    fn resize_clear_starts_at_the_active_prompt_and_preserves_history_above() {
        let mut output = Vec::new();

        clear_viewport(&mut output, 7);

        assert_eq!(output, b"\x1b[8;1H\x1b[J");
    }
}
