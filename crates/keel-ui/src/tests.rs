use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
};

use super::*;

fn snapshot(scene: &Scene, width: u16) -> String {
    let size = scene.measure(Constraints::new(width, 20));
    let area = Rect::new(0, 0, size.width.max(1), size.height.max(1));
    let mut buffer = Buffer::empty(area);
    scene.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn column_and_row_measure_natural_size() {
    let scene = Scene::new(
        Column::new().child(Text::new("Keel")).child(
            Row::new()
                .gap(1)
                .child(Text::new("a"))
                .child(Text::new("bc")),
        ),
    );
    assert_eq!(scene.measure(Constraints::new(80, 20)), Size::new(4, 2));
}

#[test]
fn composition_measures_context_multiline_body_and_input() {
    let scene = Scene::new(
        Column::new()
            .child(Text::new("context"))
            .child(Paragraph::new("first\nsecond").wrap(false))
            .child(InputLine::new("> ", "open", 4)),
    );

    assert_eq!(scene.measure(Constraints::new(80, 20)), Size::new(7, 4));
}

#[test]
fn popup_wraps_to_content_and_renders_selection() {
    let scene = Scene::new(SuggestionPopup::new(
        "1/2; Tab to accept",
        vec![
            PopupItem::new("--files-with-matches", ""),
            PopupItem::new("--files-without-match", ""),
        ],
        Some(0),
    ));
    let size = scene.measure(Constraints::new(80, 20));
    assert_eq!(size, Size::new(25, 4));
    let rendered = snapshot(&scene, 80);
    assert!(!rendered.contains("suggestions"));
    assert!(!rendered.contains("> "));
    assert!(rendered.contains("--files-with-matches"));
    assert!(rendered.contains("1/2; Tab to accept"));
}

#[test]
fn selection_reverses_only_the_term_and_keeps_details_dim() {
    let scene = Scene::new(
        SuggestionPopup::new(
            "1/1; 2ms",
            vec![PopupItem::new("alpha", "description")],
            Some(0),
        )
        .query("a"),
    );
    let size = scene.measure(Constraints::new(80, 20));
    let area = Rect::new(0, 0, size.width, size.height);
    let mut buffer = Buffer::empty(area);
    scene.render(area, &mut buffer);

    let term_style = buffer.cell((2, 1)).unwrap().style();
    let detail_style = buffer.cell((9, 1)).unwrap().style();
    assert!(term_style.add_modifier.contains(Modifier::REVERSED));
    assert!(term_style.add_modifier.contains(Modifier::UNDERLINED));
    assert!(detail_style.add_modifier.contains(Modifier::DIM));
    assert!(!detail_style.add_modifier.contains(Modifier::REVERSED));
}

#[test]
fn popup_visual_snapshot_is_stable() {
    let scene = Scene::new(SuggestionPopup::new(
        "1/2; Up/Down to select",
        vec![
            PopupItem::new("--files-with-matches", "grep option"),
            PopupItem::new("--files-without-match", "grep option"),
        ],
        Some(0),
    ));

    insta::assert_snapshot!(snapshot(&scene, 42));
}

#[test]
fn popup_limits_entries_and_renders_an_inline_scrollbar() {
    let items = (0..20)
        .map(|index| PopupItem::new(format!("item-{index:02}"), "detail"))
        .collect();
    let scene = Scene::new(
        SuggestionPopup::new("11/20; 3ms", items, Some(10)).viewport(1, MAX_VISIBLE_ITEMS),
    );
    let size = scene.measure(Constraints::new(80, 40));
    assert_eq!(size.height, 14);
    let area = Rect::new(0, 0, size.width, size.height);
    let mut buffer = Buffer::empty(area);
    scene.render(area, &mut buffer);

    let rendered = snapshot(&scene, 80);
    assert!(rendered.contains("item-01"));
    assert!(rendered.contains("item-12"));
    assert!(!rendered.contains("item-00"));
    assert!(!rendered.contains("item-13"));

    let scrollbar_x = area.x + area.width - 1;
    let thumb = buffer.cell((scrollbar_x, area.y + 5)).unwrap();
    assert_eq!(thumb.symbol(), " ");
    assert_eq!(thumb.bg, Color::White);
    assert!(!thumb.style().add_modifier.contains(Modifier::DIM));
    let track = buffer.cell((scrollbar_x, area.y + 10)).unwrap();
    assert_eq!(track.symbol(), "█");
    assert_eq!(track.fg, Color::DarkGray);
    assert_ne!(track.bg, Color::White);
    assert!(!track.style().add_modifier.contains(Modifier::DIM));
}

#[test]
fn input_line_handles_unicode_cursor_positions() {
    let scene = Scene::new(InputLine::new("keel> ", "a🙂é", 2));
    assert_eq!(scene.measure(Constraints::new(80, 3)), Size::new(10, 1));
    let rendered = snapshot(&scene, 80);
    assert!(rendered.starts_with("keel> a🙂"));
}
