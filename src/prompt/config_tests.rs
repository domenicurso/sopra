use super::*;
#[test]
fn reads_prompt_and_editor_values() -> io::Result<()> {
    let file = std::env::temp_dir().join(format!("sopra-prompt-{}.toml", std::process::id()));
    fs::write(
        &file,
        "[editor]\ncursor_animation = false\n[prompt]\nright = '[dim]{clock}[/]'\n[widgets.clock]\nformat = '%H:%M'\n",
    )?;
    let config = PromptConfig::read(&file)?;
    assert_eq!(config.cursor_animation, Some(false));
    assert_eq!(config.right.as_deref(), Some("[dim]{clock}[/]"));
    assert_eq!(config.clock_format.as_deref(), Some("%H:%M"));
    fs::remove_file(file)
}
#[test]
fn invalid_field_types_report_the_field() -> io::Result<()> {
    let doc = parse("[widgets.clock]\nformat = 42\n")?;
    let result = read_string(&doc, &["widgets", "clock", "format"]);
    assert!(
        result
            .err()
            .is_some_and(|error| error.to_string().contains("widgets.clock.format"))
    );
    let doc = parse("[prompt]\nleft = \"\\u0000\"\n")?;
    assert!(
        read_string(&doc, &["prompt", "left"])
            .err()
            .is_some_and(|error| error.to_string().contains("NUL"))
    );
    Ok(())
}
