pub(super) fn valid(name: &str) -> bool {
    if let Some(long) = name.strip_prefix("--") {
        return !long.is_empty()
            && long
                .chars()
                .any(|character| character.is_ascii_alphanumeric())
            && long
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character));
    }
    let Some(short) = name.strip_prefix('-') else {
        return false;
    };
    let count = short.chars().count();
    (count == 1
        && short
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '?'))
        || (count > 1
            && short
                .chars()
                .all(|character| character.is_ascii_alphanumeric()))
}
