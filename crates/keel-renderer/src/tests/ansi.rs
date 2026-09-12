use super::popup;
use crate::{RenderContext, Renderer};

pub(super) fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut escape = false;
    let mut csi = false;

    for byte in input.bytes() {
        if csi {
            if (0x40..=0x7e).contains(&byte) {
                csi = false;
            }
            continue;
        }
        if escape {
            if byte == b'[' {
                csi = true;
            }
            escape = false;
            continue;
        }
        if byte == 0x1b {
            escape = true;
        } else {
            output.push(byte as char);
        }
    }
    output
}

#[test]
fn encoder_uses_relative_cursor_restore_and_no_full_screen_clear() {
    let mut renderer = Renderer::new();
    let payload = renderer
        .render(
            &popup(&["run echo hi"]),
            RenderContext::new(40, 8, 5).full_repaint(true),
        )
        .unwrap()
        .transaction
        .payload;
    let payload = String::from_utf8(payload).unwrap();
    assert!(payload.starts_with("\x1b7\x1b[?25l\x1b[1B\x1b[6G"));
    assert!(payload.contains("\x1b[0m\x1b8\x1b[?25h"));
    assert!(!payload.contains(" q"));
    assert!(!payload.contains("\x1b[2J"));
}

#[test]
fn rendered_region_reports_the_reserved_height() {
    let mut renderer = Renderer::new();
    let rendered = renderer
        .render(
            &popup(&["run git"]),
            RenderContext::new(80, 12, 0).full_repaint(true),
        )
        .unwrap();
    assert_eq!(rendered.used_rows, rendered.frame.area.height);
}

#[test]
fn default_popup_uses_only_terminal_palette_colors() {
    let mut renderer = Renderer::new();
    let payload = renderer
        .render(
            &popup(&["run git", "inspect git"]),
            RenderContext::new(80, 12, 0).full_repaint(true),
        )
        .unwrap()
        .transaction
        .payload;
    let payload = String::from_utf8(payload).unwrap();

    assert!(!payload.contains("38;"));
    assert!(!payload.contains("48;"));
    assert!(payload.contains("\x1b[32m"));
    assert!(payload.contains("\x1b[97m"));
    assert!(payload.contains("\x1b[1m"));
    assert!(payload.contains("\x1b[4m"));
    assert!(payload.contains("\x1b[7m"));
}

#[test]
fn above_cursor_surface_uses_negative_row_offset_without_owning_cursor_style() {
    let mut renderer = Renderer::new();
    let rendered = renderer
        .render(
            &popup(&["run git"]),
            RenderContext::new(80, 5, 8)
                .row_offset(-5)
                .full_repaint(true),
        )
        .unwrap();
    assert_eq!(rendered.frame.row_offset, -5);
    let payload = String::from_utf8(rendered.transaction.payload).unwrap();
    assert!(payload.starts_with("\x1b7\x1b[?25l\x1b[5A"));
    assert!(!payload.contains(" q"));
}
