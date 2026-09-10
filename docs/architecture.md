# Prototype Architecture

Keel augments a live zsh prompt instead of becoming a second terminal
application. ZLE remains the only owner of interactive line editing and
terminal redisplay, which means standard typing, cursor movement, wrapping,
resize, Ctrl-C, Enter, history, and command execution keep their existing
semantics.

The Rust boundary starts with a `HostSnapshot` containing the observable ZLE
line and terminal dimensions. Rust derives UI data from that snapshot, but it
does not copy the line into an authoritative editor buffer. This avoids two
editors competing for ownership of the same cursor.

The prototype calls the Rust binary from `line-pre-redraw` and from small
delegates around common built-in edit widgets. Each delegate invokes the
original ZLE widget before requesting a prompt reset, so it observes the new
line without implementing editing itself. Rust composes a generic `keel-ui`
component tree with ratatui widgets into an offscreen `Buffer`.
`keel-renderer` serializes that one-line buffer into a zsh prompt fragment,
wrapping control sequences in `%{...%}` so zsh's width accounting stays
correct. The shell assigns the fragment to `RPROMPT` and allows the normal
redisplay to proceed.

The process boundary is intentionally replaceable. A future native module can
keep the same snapshot and fragment contracts while moving the renderer into
the zsh process; it must still avoid direct tty writes and broad widget
replacement.
