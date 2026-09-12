use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget},
};

use crate::{
    matching::match_spans,
    popup::{PopupPlacement, SuggestionPopup},
    scrollbar::render_scrollbar,
    style::{StyleToken, border_style, detail_style, footer_style, matching_style},
};

pub(crate) fn render_popup(popup: &SuggestionPopup, area: Rect, buffer: &mut Buffer) {
    if area.width < 2 || area.height < 2 {
        return;
    }

    let connector_column = popup.anchor_column.clamp(1, area.width.saturating_sub(2));
    let junction = match popup.placement {
        PopupPlacement::Below => "┴",
        PopupPlacement::Above => "┬",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style())
        .padding(Padding::horizontal(1))
        .title_bottom(
            Line::from(Span::styled(popup.footer.clone(), footer_style())).right_aligned(),
        );
    let inner = block.inner(area);
    block.render(area, buffer);

    let junction_column = area.x + connector_column;
    let junction_row = match popup.placement {
        PopupPlacement::Below => area.y,
        PopupPlacement::Above => area.y + area.height - 1,
    };
    if let Some(cell) = buffer.cell_mut((junction_column, junction_row)) {
        cell.set_symbol(junction);
        cell.set_style(border_style());
    }

    let visible_count = popup
        .items
        .len()
        .min(popup.max_visible_items)
        .min(inner.height as usize);
    let mut viewport_start = popup
        .viewport_start
        .min(popup.items.len().saturating_sub(visible_count));
    if let Some(selected) = popup.selected {
        if selected < viewport_start {
            viewport_start = selected;
        } else if selected >= viewport_start.saturating_add(visible_count) && visible_count > 0 {
            viewport_start = selected.saturating_add(1).saturating_sub(visible_count);
        }
    }

    let end = viewport_start
        .saturating_add(visible_count)
        .min(popup.items.len());
    let mut lines = Vec::with_capacity(visible_count);
    for (offset, item) in popup.items[viewport_start..end].iter().enumerate() {
        let selected = popup.selected == Some(viewport_start + offset);
        let term_style = if selected {
            StyleToken::Selection.style()
        } else {
            Style::default()
        };
        let mut spans = match_spans(
            &item.label,
            &popup.query,
            &item.match_indices,
            term_style,
            matching_style(selected),
        );
        if !item.detail.is_empty() {
            spans.push(Span::styled(format!("  {}", item.detail), detail_style()));
        }
        lines.push(Line::from(spans));
    }

    Paragraph::new(lines).render(inner, buffer);
    if popup.items.len() > popup.max_visible_items {
        render_scrollbar(
            Rect::new(
                area.x + area.width.saturating_sub(1),
                area.y.saturating_add(1),
                1,
                area.height.saturating_sub(2),
            ),
            popup.items.len(),
            viewport_start,
            visible_count,
            buffer,
        );
    }
}
