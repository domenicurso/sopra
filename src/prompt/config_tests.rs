use super::*;
#[test]
fn reads_and_updates_prompt_templates_without_losing_comments() -> io::Result<()> {
    let file = std::env::temp_dir().join(format!("sopra-prompt-{}.toml", std::process::id()));
    fs::write(
        &file,
        "# keep me\n[prompt]\nright = '[dim]{clock}[/]' # inline comment\n[widgets.clock]\nformat = '%H:%M'\n[widgets.git_status]\nformat = '<{items}>'\nseparator = ' / '\n",
    )?;
    let mut config = PromptConfig::read(&file)?;
    assert_eq!(config.right.as_deref(), Some("[dim]{clock}[/]"));
    assert_eq!(config.git_status_format.as_deref(), Some("<{items}>"));
    assert_eq!(config.git_status_separator.as_deref(), Some(" / "));
    config.right = Some("{git}".into());
    update(&file, &config)?;
    let text = fs::read_to_string(&file)?;
    assert!(text.contains("# keep me") && text.contains("# inline comment"));
    assert_eq!(PromptConfig::read(&file)?, config);
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
