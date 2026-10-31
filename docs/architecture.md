# Runtime ownership

Zsh owns execution and shell state. Its bridge starts one private Rust worker,
passes immutable environment and history snapshots over a Unix socket, and accepts
the returned buffer or directory action. Zsh does not parse configuration or know
widget names. Rust returns opaque, correctly escaped native prompts for the brief
ZLE handoff and secondary-shell fallback.

The worker session owns validated configuration, history state, the completion
engine, widget instances, the syntax checker, and the terminal palette. An edit
borrows these services, owns its buffer and interaction state, then returns the
services before Zsh executes the command. Terminal input modes are acquired for
editing and restored at that boundary. Shell exit closes the worker and its widget
processes.

`src/config` is the sole configuration/defaults boundary. `src/prompt/template`
compiles named style scopes, widget references, and implicit connectors. Widgets
produce structured text; their output never passes through the template parser.
The SDK contains no terminal color policy. Templates supply color, while widget
parts can express dimming, emphasis, and selection.

`src/actions` translates physical keys into editor actions. Completion focus is an
explicit none/pending/selected state beside its result set and viewport. The shared
`src/text` layout maps prompt and command graphemes to cells; drawing, mouse hit
testing, and cursor positioning consume that map. The renderer owns terminal
painting and resize/reflow recovery separately from logical text layout. Resize
bursts are coalesced for one frame interval before querying the terminal and
painting, so intermediate dimensions do not continually invalidate cursor replies.

`sopra-shell-context` owns environment snapshots and command discovery. Syntax
highlighting and completion use that boundary directly. Completion owns discovery
of command help and its graph cache, filesystem candidates, and ranking. Queued
help requests capture their environment so subsequent shell commands cannot change
an in-flight subprocess's inputs. Generation boundaries reject results from an
older edit while retaining reusable command graphs.

Widgets implement the single `sopra-widget-sdk` contract. Built-ins link the crate;
custom single-file Rust widgets compile against embedded copies of those same
sources. Each widget is retained by the session and executes callbacks behind a
background worker; the editor reads cached text. Custom processes have callback,
compilation, and response-size limits, and report errors independently of editing.
Source changes rebuild the affected widget. Extension behavior and limits are
specified in [the widget SDK reference](widgets.md).

Submission checking uses an isolated Zsh with execution disabled, behind a retained
background service. Complete commands reuse its parser process; continuing input
feeds the existing parse, and editing an earlier incomplete prefix restarts it.
The service returns a continuation decision or an I/O error. Input typed during
that decision stays queued in the terminal so it cannot overtake the newline or
be consumed from the next foreground program. Painting and resizing continue.

The public CLI uses a framed control protocol on that shell's existing Unix
socket. A listener thread handles health requests independently of editing and
admits one stateful request at a time; changes during editing are rejected before
queuing, so timed-out clients cannot trigger delayed mutations. The session owner
applies config changes between edits. Refresh validates before reconciling widget
instances and commits the new config only after reconciliation succeeds. Config
checks use the same parser and widget catalog without starting a runtime.

Restart validates the config, acknowledges the request, then executes the worker
binary over the existing process. The owning shell keeps its PID and socket path;
clients wait for a new startup generation before reporting success. Widget pipes
close during process replacement and their state resets. Shell history and command
execution remain owned by Zsh. Widget inspection uses the catalog plus retained
instances, including errors from sources removed after loading.


## Hosted session boundary

`src/host` owns the outer terminal and a child Zsh PTY in `sopra shell` mode.
The existing Zsh bridge and persistent editor worker run against the child PTY;
Zsh still owns execution, environment changes, jobs, and command history. A
bootstrap directory loads the user's startup files without modifying them.

A bounded reader queue feeds a vt100 terminal model. A separate writer drains
queued input, so a large paste cannot block output processing; pending input
applies backpressure to outer-terminal reads. The host answers child geometry
queries from its model and forwards terminal palette, title, and clipboard
operations across the outer boundary. Foreground alternate-screen mouse input
is routed to the child's requested protocol.

The same terminal grid supplies the live screen and scrollback viewport. Wheel
and page keys move that viewport without adding UI or relocating the prompt.
Typing returns to the live screen before input reaches the child; mouse clicks
at the live bottom use the child's requested protocol. The configured line limit
bounds in-memory scrollback independently of Zsh's persisted command history.

The host owns terminal mode restoration and hangs up the foreground process
group and Zsh on shutdown. Parser or I/O errors terminate the hosted session
through the same cleanup path. Supported controls and compatibility limits are
documented in [hosted shells](hosted-shell.md).
