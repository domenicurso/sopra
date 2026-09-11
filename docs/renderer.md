# Renderer Contract

keel-ui provides a small declarative composition layer over ratatui:

```rust
pub trait Component {
    fn measure(&self, constraints: Constraints) -> Size;
    fn render(&self, area: Rect, buffer: &mut ratatui::buffer::Buffer);
}
```

Scene owns the root component. Built-ins cover the common prompt surfaces,
while ratatui_component lets a caller place any cloneable ratatui Widget with
an explicit measured size. This keeps prompt composition generic: adding a
status panel, separator, inline hint, or future overlay adds a component tree
child rather than another prompt-specific renderer branch.

The renderer allocates an offscreen ratatui Buffer at the requested terminal
width and the scene's measured height. RenderedFrame retains the complete
buffer, used content bounds, and measured size. Renderer::diff compares the
current frame with the previous one and reports changed cells, changed rows,
and rows that must be cleared when the scene shrinks.

For the zsh adapter, Renderer::to_zsh_prompt serializes the used rows as a
compact prompt fragment, preserving per-cell foreground/background colors and
bold styling with zsh %{...%} control markers. Rows are trimmed independently,
so a full-width header cannot introduce trailing spaces into the actual input
prefix. Renderer::to_ansi is the raw terminal form for future region-scoped
patching. Neither path clears the screen, enters the alternate screen, moves
the cursor, or takes ownership of scrollback. The augmentation service returns
separate left and right prompt surfaces plus native cursor and syntax
decorations.

The default scene is a right-aligned status badge built from Row, Text, Panel,
and Align. The live POC profiles add a Rust-composed multiline prompt header,
a full-width Rule, syntax spans, and a direct ratatui LineGauge widget. Their
status is derived from the live host snapshot, and the persistent server
caches an unchanged snapshot instead of repainting it. Completion popups and
richer semantic overlays remain later features because they need a host-safe
placement contract that does not fight ZLE's own cursor and wrapping model.
