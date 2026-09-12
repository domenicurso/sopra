# Renderer

The renderer has one pipeline:

```text
Component tree -> measure -> ratatui Buffer -> FrameDiff -> RenderTransaction -> ANSI bytes
```

`keel-ui` is responsible for composition and measurement. A scene reports its natural size under terminal constraints, then paints into a ratatui `Buffer`. The renderer never interprets prompt strings or asks ZLE to display a second prompt.

Every frame has a surface-local buffer and a terminal-relative origin column. The origin is the host cursor column, and the first painted row is the row immediately below the saved host cursor. This coordinate system makes an autocomplete box an ordinary child surface while keeping the host prompt untouched.

The diff compares cells, dimensions, and origin. Changed rows are erased and repainted, a moved surface first clears its old origin, and rows that disappear are explicitly erased. An unchanged frame produces no payload unless the caller requests a full repaint. The native module requests a full repaint after each ZLE redraw because its pre-redraw phase has already removed the old surface from the terminal.

The ANSI writer only emits save/restore cursor, cursor visibility, horizontal column moves, relative row moves, targeted erase operations, style changes, and row text. It never emits a screen clear, alternate-screen transition, scrollback operation, or write outside the surface rows.

The current MVP renders a rounded, styled suggestion box below the line. The component API already accepts generic ratatui widgets, so later completion panels, status surfaces, and cursor-anchored overlays can share the same measurement, buffer, diff, and patch pipeline without another terminal writer.
