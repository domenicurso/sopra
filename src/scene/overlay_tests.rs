use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use sopra_completion::{CompletionItem, CompletionKind};

use super::{
    Element,
    canvas::{Canvas, TextRun},
    overlay::{OverlayElement, overlay_width},
};

#[test]
fn width_follows_content_until_the_terminal_edge_or_cap() {
    let mut items = vec![CompletionItem::new(
        "a-very-long-command-name",
        "a description that needs room",
        "a-very-long-command-name",
        CompletionKind::Generic,
    )];
    assert_eq!(overlay_width(&items, "", "", 120), 59);
    assert_eq!(overlay_width(&items, "", "", 32), 30);
    assert_eq!(overlay_width(&items, &"x".repeat(200), "", 240), 100);
    items[0].description = Some("x".repeat(200));
    assert_eq!(overlay_width(&items, "", "", 240), 100);
}

#[test]
fn popup_clears_underlying_text_and_styles_through_its_padding_and_borders() {
    let (mut canvas, underlying) = canvas_with_selected_text();
    let area = Rect::new(3, 1, 24, 4);
    OverlayElement {
        area,
        items: vec![CompletionItem::new(
            "alpha",
            "",
            "alpha",
            CompletionKind::Generic,
        )],
        selected: None,
        viewport_start: 0,
        item_queries: vec![String::new()],
        footer_hint: String::new(),
        footer: String::new(),
        connector_column: 2,
        connector: "┴",
    }
    .paint(&mut canvas);
    for row in area.top()..area.bottom() {
        for column in area.left()..area.right() {
            let cell = &canvas.buffer[(column, row)];
            assert_ne!(cell.symbol(), "X");
            assert_eq!(cell.bg, Color::Reset);
            assert!(
                !cell
                    .modifier
                    .intersects(Modifier::REVERSED | Modifier::BOLD)
            );
        }
    }
    assert_eq!(canvas.buffer[(5, 2)].symbol(), "a");
    assert_eq!(canvas.buffer[(4, 3)].symbol(), " ");
    assert_eq!(canvas.buffer[(2, 2)].symbol(), "X");
    assert_eq!(canvas.buffer[(2, 2)].modifier, underlying.add_modifier);
}

fn canvas_with_selected_text() -> (Canvas, Style) {
    let mut canvas = Canvas::new(Rect::new(0, 0, 30, 6));
    let underlying = Style::default()
        .fg(Color::Red)
        .bg(Color::Yellow)
        .add_modifier(Modifier::REVERSED | Modifier::BOLD);
    for row in 0..6 {
        canvas.text(TextRun {
            position: (0, row),
            text: "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
            style: underlying,
            max_width: 30,
        });
    }
    (canvas, underlying)
}
