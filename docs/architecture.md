# Augmentation Architecture

Keel augments a live zsh prompt instead of becoming a second terminal
application. ZLE remains the only owner of interactive line editing and
terminal redisplay, so standard typing, cursor movement, wrapping, resize,
Ctrl-C, Enter, history, and command execution keep their existing semantics.

The shell boundary is a single persistent renderer process:

```text
line-pre-redraw / precmd / preexec
                |
                v
      tab-delimited HostSnapshot
                |
                v
      keel-augment --server
        keel-core -> keel-ui -> ratatui Buffer -> keel-renderer
                |
                v
      zsh host decoration plan
   PROMPT/RPROMPT       cursor/syntax spans
```

HostSnapshot contains the observable ZLE buffer, cursor, terminal dimensions,
keymap, current working directory, and last command status. Rust treats that
data as input to the augmentation scene; it does not copy the line into a
competing editor. The long-lived process keeps its renderer, previous frame,
scheduler clock, and statistics between requests, which removes process
startup from the keypress path and makes unchanged snapshots cheap cache hits.

keel-ui owns a generic component tree. Text, Paragraph, InputLine, Row,
Column, Align, Spacer, and Panel are convenience components, and
ratatui_component adapts cloneable ratatui Widget values directly. Each component
measures itself under constraints and renders into the same offscreen
ratatui Buffer; keel-renderer then finds the used content bounds, serializes
styles into ANSI controls, and wraps controls with zsh %{...%} markers so
zsh's width accounting remains correct.

The shell adapter installs only line-pre-redraw, line-finish, precmd, and
preexec hooks. It never wraps built-in ZLE widgets or mutates BUFFER or CURSOR.
When a returned prompt surface changes, it asks ZLE for its normal prompt
refresh; a re-entry guard prevents the refresh from recursively triggering
another loop. The only direct terminal sequence is an optional zero-width
cursor-style change such as blink-block or bar. It does not move the cursor or
write screen cells, so it cannot desynchronise ZLE's coordinate model.

The optional `KEEL_AUGMENT_CURSOR_MODE=highlight` path adds one
`region_highlight` span for the grapheme under the cursor. The span is removed
before each replacement and on disable, which lets other zsh highlighters keep
their own entries. This is the practical cursor-relative layer: Rust computes
the decoration, while zsh applies it through the host's own redisplay API.

Raw cursor movement remains intentionally outside the production contract.
An ANSI transaction that saves the cursor, paints cells, and restores it can
be added later for a narrowly scoped widget, but putting that transaction in
`RPROMPT` would make prompt-width accounting and ZLE redisplay undefined.

The process boundary is deliberately replaceable. A future native module may
move the same HostSnapshot, component, and rendered-fragment contracts into
the zsh process, but it must keep zsh as the owner of editing and terminal
semantics unless a later design proves a narrower ownership transfer is safe.
