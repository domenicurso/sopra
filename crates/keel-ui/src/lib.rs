mod basic;
mod component;
mod geometry;
mod input;
mod layout;
mod matching;
mod paragraph;
mod popup;
mod popup_render;
mod ratatui_component;
mod scrollbar;
mod style;

pub use basic::{Spacer, Text};
pub use component::{Component, Scene};
pub use geometry::{Constraints, Size};
pub use input::InputLine;
pub use layout::{Align, Column, HorizontalAlignment, Row};
pub use paragraph::Paragraph;
pub use popup::{MAX_VISIBLE_ITEMS, PopupItem, PopupPlacement, SuggestionPopup};
pub use ratatui_component::{RatatuiComponent, ratatui_component};
pub use style::StyleToken;

#[cfg(test)]
mod tests;
