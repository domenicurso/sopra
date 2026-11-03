// Completion values are shell arguments; subcommand phrases have a different insertion contract.
pub(crate) fn quote(value: &str, input: &str) -> String {
    if input.starts_with('\'') {
        return format!("'{}'", value.replace('\'', "'\\''"));
    }
    if input.starts_with('"') {
        let escaped = value
            .chars()
            .flat_map(|character| {
                if "\\\"$`".contains(character) {
                    vec!['\\', character]
                } else {
                    vec![character]
                }
            })
            .collect::<String>();
        return format!("\"{escaped}\"");
    }
    value
        .chars()
        .flat_map(|character| {
            if character.is_whitespace() || "\\\"'$`!;&|()<>{}[]*?#~".contains(character) {
                vec!['\\', character]
            } else {
                vec![character]
            }
        })
        .collect()
}
