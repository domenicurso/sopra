# Renderer Contract

`keel-ui` exposes a small component interface:

```rust
pub trait Component {
    fn measure(&self, max_width: u16) -> Size;
    fn render(&self, area: Rect, buffer: &mut ratatui::buffer::Buffer);
}
```

The `Scene` owns a component tree, while built-in components use ratatui's
`Paragraph` and `Block` widgets. `keel-renderer` allocates an offscreen buffer
for the measured scene, asks the scene to render, and adapts the resulting
cells to zsh prompt syntax.

This first slice emits a single right-side badge because that gives us a
visible, styled proof of the boundary without taking over the editing line.
Multiline prompt decoration and completion surfaces can be added as new
components once their display contract is defined in terms zsh can safely
redisplay.
