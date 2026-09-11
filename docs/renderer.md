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

For the zsh adapter, Renderer::to_zsh_prompt serializes only the used bounds
as a single prompt fragment, preserving per-cell foreground/background colors
and bold styling with zsh %{...%} control markers. Renderer::to_ansi is the
raw terminal form for future region-scoped patching. Neither path clears the
screen, enters the alternate screen, moves the cursor, or takes ownership of
scrollback. The augmentation service wraps the fragment in a host plan that
can additionally request a native cursor style and a zsh `region_highlight`
span.

The current production-shaped scene is a right-aligned status badge built
from Row, Text, Panel, and Align. Its status is derived from the live host
snapshot, and the persistent server caches an unchanged snapshot instead of
repainting it. The component API already supports multiline paragraphs,
wrapped input measurement, cursor painting, borders, padding, and direct
ratatui widgets; completion popups and richer semantic overlays remain later
features because they need a host-safe placement contract first.
