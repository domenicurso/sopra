use super::probe::Parser;
fn needs_continuation(code: &str) -> std::io::Result<bool> {
    Parser::default().check(code)
}

#[test]
fn zsh_distinguishes_continuations_from_complete_and_invalid_commands() -> std::io::Result<()> {
    for code in [
        "echo \\",
        "echo 'hello",
        "for x in a; do",
        "echo hi |",
        "cat <<EOF",
        "cat <<EOF\nhello",
    ] {
        assert!(needs_continuation(code)?, "{code:?}");
    }
    for code in [
        "echo hello",
        "echo )",
        "for x in a; do\necho $x\ndone",
        "cat <<EOF\nhello\nEOF",
        "echo \\\nhello",
    ] {
        assert!(!needs_continuation(code)?, "{code:?}");
    }
    Ok(())
}

#[test]
fn syntax_check_does_not_execute_expansions_or_commands() -> std::io::Result<()> {
    let file = std::env::temp_dir().join(format!("sopra-noexec-{}", std::process::id()));
    let code = format!(
        "echo $(touch '{}'); touch '{}'",
        file.display(),
        file.display()
    );
    assert!(!needs_continuation(&code)?);
    assert!(!file.exists());
    Ok(())
}

#[test]
fn persistent_parser_reuses_complete_input_and_resets_edited_continuations() -> std::io::Result<()>
{
    let mut parser = Parser::default();
    assert!(!parser.check("echo first")?);
    assert!(!parser.check("echo second")?);
    assert!(parser.check("for x in a; do")?);
    assert!(parser.check("for x in a; do\necho $x")?);
    assert!(!parser.check("for x in a; do\necho $x\ndone")?);
    assert!(parser.check("echo '")?);
    assert!(!parser.check("echo changed")?);
    Ok(())
}
