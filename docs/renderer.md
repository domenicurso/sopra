# Renderer

The renderer has one pipeline:

```text
Component tree -> measure -> ratatui Buffer -> FrameDiff -> RenderTransaction -> ANSI bytes
```

`keel-ui` is responsible for composition and measurement. A scene reports its natural size under terminal constraints, then paints into a ratatui `Buffer`. The renderer never interprets prompt strings or asks ZLE to display a second prompt.

Every frame has a surface-local buffer, a terminal-relative origin column, and a signed row offset from the saved host cursor. The popup origin is computed from the completion token and clamped to the terminal width; its top junction points at the host cursor while the body is painted one row below, or above the cursor when the lower terminal region cannot fit the measured height. This coordinate system makes an autocomplete box an ordinary child surface while keeping the host prompt untouched.

The diff compares cells, dimensions, origin, and row offset. Changed rows are erased and repainted, a moved surface first restores the saved cursor before clearing its old origin, and rows that disappear are explicitly erased. An unchanged frame produces no payload unless the caller requests a full repaint. The native module requests a full repaint after each ZLE redraw because its pre-redraw phase has already removed the old surface from the terminal.

The ANSI writer only emits save/restore cursor, cursor visibility, horizontal column moves, relative row moves, targeted erase operations, style changes, and row text. It never emits a screen clear, alternate-screen transition, scrollback operation, or write outside the surface rows. Cursor shape is deliberately outside this writer: the native shim emits the terminal's block-cursor sequence for the active ZLE line and restores the terminal cursor mode on finish.

The current MVP renders a rounded suggestion box below the line, or above it when the lower region is too short. Entries come from the host Zsh completion system, are fuzzy-ranked in `keel-core`, and are capped at twelve visible rows with one row of travel room. The scrollbar is painted into the popup's right border, while the count and fetch time are painted into the bottom border. The component API accepts generic ratatui widgets, so later completion panels, status surfaces, and cursor-anchored overlays can share the same measurement, buffer, diff, and patch pipeline without another terminal writer.
