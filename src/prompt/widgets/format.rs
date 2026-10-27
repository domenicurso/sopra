use super::Widget;

#[derive(Clone, Copy)]
enum Block {
    Condition(bool),
    Style,
}

pub(super) fn render(template: &str, widgets: &[Box<dyn Widget>]) -> String {
    let mut output = String::new();
    let mut remaining = template;
    let mut blocks = Vec::new();
    while !remaining.is_empty() {
        if let Some(length) = consume_tag(remaining, widgets, &mut blocks, &mut output) {
            remaining = &remaining[length..];
        } else if let Some(widget) = widgets
            .iter()
            .find(|item| remaining.starts_with(item.token()))
        {
            if visible(&blocks) {
                let value = widget.render();
                output.push_str(&if widget.markup() {
                    value
                } else {
                    value.replace('[', "[[")
                });
            }
            remaining = &remaining[widget.token().len()..];
        } else if let Some(character) = remaining.chars().next() {
            if visible(&blocks) {
                output.push(character);
            }
            remaining = &remaining[character.len_utf8()..];
        }
    }
    output
}

fn consume_tag(
    input: &str,
    widgets: &[Box<dyn Widget>],
    blocks: &mut Vec<Block>,
    output: &mut String,
) -> Option<usize> {
    if input.starts_with("[[") {
        if visible(blocks) {
            output.push_str("[[");
        }
        return Some(2);
    }
    let end = input.strip_prefix('[')?.find(']')? + 1;
    let tag = &input[1..end];
    if let Some(name) = tag.strip_prefix("if ") {
        let token = format!("{{{name}}}");
        let present = widgets
            .iter()
            .find(|widget| widget.token() == token)
            .is_some_and(|widget| !widget.render().is_empty());
        blocks.push(Block::Condition(present));
    } else if tag == "/" {
        let before = visible(blocks);
        if matches!(blocks.pop()?, Block::Style) && before {
            output.push_str("[/]");
        }
    } else if crate::prompt::markup::style_tag(tag).is_some() {
        if visible(blocks) {
            output.push_str(&input[..=end]);
        }
        blocks.push(Block::Style);
    } else {
        return None;
    }
    Some(end + 1)
}

fn visible(blocks: &[Block]) -> bool {
    blocks
        .iter()
        .all(|block| !matches!(block, Block::Condition(false)))
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::prompt::{parse, widgets::Widget};
    use ratatui::style::Modifier;

    struct Text(&'static str);
    impl Widget for Text {
        fn token(&self) -> &'static str {
            "{git}"
        }
        fn render(&self) -> String {
            self.0.into()
        }
    }

    #[test]
    fn conditional_separator_depends_on_widget_content() {
        let template = "[if git]{git}[dim] : [/][/]{missing}";
        let absent: Vec<Box<dyn Widget>> = vec![Box::new(Text(""))];
        let present: Vec<Box<dyn Widget>> = vec![Box::new(Text("main"))];
        assert_eq!(render(template, &absent), "{missing}");
        let spans = parse(&render(template, &present));
        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            "main : {missing}"
        );
        assert!(spans[1].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn widget_output_is_never_interpreted_as_markup() {
        let widgets: Vec<Box<dyn Widget>> = vec![Box::new(Text("[red]branch[/]"))];
        let spans = parse(&render("{git}", &widgets));
        assert_eq!(spans[0].text, "[red]branch[/]");
        assert_eq!(spans[0].style.fg, None);
    }
}
