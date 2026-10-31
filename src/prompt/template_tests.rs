use super::*;
use ratatui::style::{Color, Modifier};
use std::collections::BTreeMap;

#[test]
fn styles_are_scoped_and_widgets_are_literal() -> io::Result<()> {
    let template = Template::compile("[bold blue]$path[/bold blue] [dim]in[/dim] $$")?;
    let values = BTreeMap::from([("path".into(), "[red]literal[/red]".into())]);
    let spans = template.render(&values);
    assert_eq!(spans[0].text, "[red]literal[/red]");
    assert_eq!(spans[0].style.fg, Some(Color::Blue));
    assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    assert_eq!(spans.last().map(|s| s.text.as_str()), Some(" $"));
    for invalid in ["[red]x[/blue]", "[red]x", "[/red]", "[bold blue]x[/bold]"] {
        assert!(Template::compile(invalid).is_err());
    }
    Ok(())
}

#[test]
fn connectors_only_join_visible_values_and_keep_outer_text() -> io::Result<()> {
    let template = Template::compile("$a · [dim]$b[/dim] · $c\n$ ")?;
    for (a, b, c, expected) in [
        ("a", "b", "c", "a · b · c\n$ "),
        ("", "b", "c", "b · c\n$ "),
        ("a", "", "c", "a · c\n$ "),
        ("a", "", "", "a\n$ "),
        ("", "", "", "\n$ "),
    ] {
        let values = BTreeMap::from([
            ("a".into(), a.into()),
            ("b".into(), b.into()),
            ("c".into(), c.into()),
        ]);
        assert_eq!(
            template
                .render(&values)
                .iter()
                .map(|s| s.text.as_str())
                .collect::<String>(),
            expected
        );
    }
    Ok(())
}
