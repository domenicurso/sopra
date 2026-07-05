# Keel Architecture Specification

## 1. Purpose

Keel is a native command-line interface layer for zsh. It replaces the interactive editing, prompt, completion, and rendering experience while preserving zsh as the shell runtime. The system is designed around a custom rendered input surface with frame-based updates, high-quality cursor animation, transient prompts, rich completions, responsive layout, mouse-aware editing, and deep access to zsh shell semantics.

The product exists at the boundary between a shell and a terminal UI runtime. Zsh remains responsible for shell behavior. Keel is responsible for the human-facing command-line experience.

## 2. Architectural Thesis

Keel treats zsh as a backend for command execution, shell state, expansion behavior, completion knowledge, history semantics, functions, aliases, and environment context. Keel owns the interactive frontend: prompt rendering, input editing, cursor rendering, completion presentation, selection, history browsing, syntax highlighting, mouse interaction, terminal responsiveness, and frame scheduling.

The core architecture is a native zsh module backed by a Rust runtime. The native module provides same-process integration with zsh. The Rust runtime owns the editor, renderer, prompt system, completion interface, and event loop.

The high-level model is:

- The user interacts with Keel.
- Keel renders the command-line interface.
- Keel asks zsh for shell-specific knowledge.
- Keel returns accepted commands to zsh.
- Zsh parses and executes commands normally.
- Keel resumes control at the next interactive prompt.

## 3. Product Boundary

Keel replaces the interactive command-line surface, not the shell language. Its authority begins when zsh is ready to read an interactive command and ends when a submitted command is handed back to zsh for execution.

Keel owns:

- Prompt rendering
- Right prompt rendering
- Transient prompt rendering
- Input buffer editing
- Cursor movement and cursor animation
- Selection and clipboard behavior within the command line
- Undo and redo
- Completion UI
- Completion ranking and filtering
- History search UI
- Autosuggestions
- Syntax highlighting
- Mouse interaction during editing
- Terminal resize responsiveness
- Frame scheduling
- Async prompt widgets
- Terminal drawing and frame diffing

Zsh owns:

- Command parsing
- Command execution
- Shell expansion
- Globbing
- Aliases
- Functions
- Shell variables
- Environment state
- Working directory state
- Job control
- Shell history storage
- Native completion knowledge
- Exit status reporting

## 4. Native zsh Module

Keel is loaded into zsh as a native module. The module is the bridge between zsh and the Rust runtime. It integrates Keel with zsh’s interactive lifecycle while keeping the shell-facing surface small and controlled.

The native module provides:

- Module initialization and teardown
- Activation and deactivation of Keel’s interactive editor
- Access to zsh state required by the runtime
- An interactive command-read integration point
- Completion bridge entry points
- Prompt lifecycle integration
- Shell state synchronization
- Panic and failure boundaries around Rust calls

The module remains thin. It adapts zsh’s internal world to a stable Rust-facing interface. The Rust runtime contains the actual product behavior.

The precise interception point for interactive command reading is an implementation research area. Early versions may use ZLE-level takeover points to prove behavior, while the target design is a deeper same-process command-line replacement that uses zsh for shell semantics and Keel for the rendered input surface.

## 5. Rust Runtime

The Rust runtime is the core of Keel. It owns the editor, renderer, layout engine, completion model, frame scheduler, prompt system, terminal driver, cache model, and async state model.

The runtime is organized around deterministic state transitions:

- Current application state
- Incoming event
- Updated application state
- Rendered frame
- Terminal diff
- Terminal patch

This structure allows prompt rendering, cursor animation, completion updates, async widgets, and resizing to share one rendering pipeline instead of being modeled as independent shell hooks.

## 6. Technology Choices

Keel is primarily a Rust project with a small native boundary for zsh integration. The zsh-facing layer may use C or a C-compatible ABI shim, while the rendering, editing, completion, prompt, and runtime systems live in Rust.

Recommended implementation components:

- Rust as the primary implementation language for the runtime, renderer, editor, completion engine, caches, and diagnostics.
- A small C or C-compatible native module boundary for zsh loading, zsh state access, and shell lifecycle integration.
- Ratatui for structured terminal UI layout and reusable terminal widgets where it fits Keel’s custom frame model.
- Crossterm or termwiz for terminal control, key input, mouse input, resize events, truecolor output, cursor visibility, and terminal capability handling.
- unicode-width and unicode-segmentation for accurate cell-width measurement, grapheme handling, cursor movement, selection, truncation, and wrapping.
- ropey or a similar rope-backed text buffer for editable multiline command buffers.
- nucleo or a similar fuzzy matcher for history search, completion filtering, and candidate ranking.
- tokio or a comparable async runtime for background prompt state, completion source refreshes, filesystem work, cache updates, and timed render events.
- serde with a compact serialization format for internal protocols, diagnostic snapshots, cache data, and module-runtime state exchange.
- gitoxide, libgit2, or direct Git metadata reading for fast repository state without relying on slow synchronous shell commands.
- notify or a platform-specific filesystem watcher for cache invalidation when project files, repositories, or completion metadata change.
- tracing and structured logs for render timing, module lifecycle events, completion bridge behavior, cache behavior, and crash diagnostics.
- portable-pty for integration testing, shell-behavior test harnesses, and possible fallback experiments, rather than as the primary architecture.

These choices are implementation recommendations, not architectural dependencies. The architecture depends on a native zsh bridge, a Rust runtime, a deterministic render pipeline, and a custom command-line editor. Individual libraries can be swapped if they no longer match the runtime requirements.

## 7. Runtime State Model

Keel maintains a complete model of the interactive command-line surface.

Primary runtime state includes:

- Editing mode
- Input buffer
- Cursor position
- Animated cursor position
- Selection range
- Undo and redo stacks
- Prompt state
- Right prompt state
- Transient prompt state
- Completion state
- Autosuggestion state
- History search state
- Syntax highlighting state
- Terminal size
- Mouse state
- Shell state
- Async widget state
- Last command status
- Last command duration
- Current working directory
- Git or project context
- Terminal capabilities
- Frame timing state

The renderer reads this state and produces a complete visual frame for the command-line region.

## 8. Mode Model

Keel operates through explicit modes. Each mode defines who owns input, who owns rendering, and how events are interpreted.

Core modes include:

- Prompt editing mode
- Completion mode
- History search mode
- Selection mode
- Command submission mode
- Command execution handoff mode
- Recovery mode

Prompt editing mode is the default interactive state. Keel owns input and rendering.

Completion mode extends prompt editing with a completion menu, candidate filtering, candidate preview, and selection behavior.

History search mode provides a richer interface for navigating prior commands while still returning an editable command buffer.

Command submission mode performs the final render of the accepted line and prepares the command for zsh execution.

Command execution handoff mode transfers control to zsh so the submitted command can execute with normal shell behavior.

Recovery mode restores terminal state after render failures, interrupted operations, terminal resize edge cases, native module errors, or runtime failures.

## 9. Rendering System

Keel uses a frame-based rendering system. Rendering is based on application state, time, and terminal size. The renderer produces a virtual frame representing the desired command-line UI, diffs it against the previous frame, and writes only the required terminal updates.

The renderer is responsible for:

- Prompt layout
- Right prompt layout
- Input buffer layout
- Completion menu layout
- History UI layout
- Tooltip layout
- Cursor rendering
- Selection rendering
- Syntax highlighting
- Autosuggestion rendering
- Truncation
- Wrapping control
- Width calculation
- Unicode handling
- Frame diffing
- Terminal patch generation

The terminal receives deliberate patches generated by the renderer. The renderer tracks visible width, prompt height, input height, completion height, and cursor location so that redraws remain stable across editing, resizing, and animation.

## 10. Frame Scheduler

Keel uses a frame scheduler rather than hook-based redraws. Hooks and shell lifecycle events feed the runtime, but they do not define rendering frequency.

The scheduler supports multiple update rates depending on visible state:

- Static prompt state uses no continuous redraw.
- Clock widgets use time-based redraws.
- Loading widgets use moderate frame rates.
- Cursor movement animations use high frame rates during motion.
- Cursor blink animations use scheduled blink-phase updates.
- Completion menu animations use higher rates while actively changing.
- Resize events trigger immediate layout recomputation.

The frame scheduler is dirty-state aware. It renders when visible state changes, when a timed animation advances, or when terminal dimensions change. This creates an FPS-capable UI without constantly repainting when nothing is changing.

## 11. Cursor Rendering and Animation

Keel hides the native terminal cursor during editing and renders its own virtual cursor. This allows cursor behavior to be controlled by the runtime rather than the terminal emulator.

The cursor model separates logical cursor position from visual cursor position. Logical position is the actual insertion point in the buffer. Visual position is the animated cursor location currently being drawn.

Cursor rendering supports:

- Smooth movement between cells
- Blink timing
- Opacity-style fade simulation
- Style variants such as block, beam, underline, and hybrid forms
- Cursor color control
- Animation timing
- Cursor behavior during selection
- Cursor behavior during completion
- Cursor behavior during transient rendering

Because terminals do not provide browser-style alpha compositing for text cells, opacity is simulated through color blending against the known or configured background. The result is visually similar to an opacity animation while remaining compatible with terminal cell rendering.

## 12. Prompt System

Keel’s prompt is rendered by the same runtime as the editor. It is not a separate shell prompt theme.

The prompt system supports:

- Left prompt
- Right prompt
- Multiline prompt layouts
- Prompt widgets
- Async prompt state
- Time-based prompt widgets
- Git and project context
- Exit status
- Command duration
- Current directory display
- Runtime context where relevant
- Terminal-width-aware truncation
- Stable layout under resize
- Final transient prompt rendering

Prompt segments are semantic objects. They describe their content, style, priority, minimum width, ideal width, and truncation behavior. The layout engine decides what fits in the current terminal width.

## 13. Transient Prompt System

Keel maintains separate active and final prompt layouts.

The active prompt is the full interactive prompt shown while the user is editing. It may contain full context such as directory, git status, command duration, right prompt content, clocks, and async widgets.

The final prompt is the compact scrollback representation rendered when a command is accepted. It preserves command history readability while removing transient editing UI.

The transient prompt system is part of the render lifecycle. On command submission, Keel replaces the active prompt region with the final prompt representation and accepted command, then hands the command to zsh for execution.

## 14. Responsive Layout

Keel performs layout before writing to the terminal. Prompt and editor content are measured, truncated, wrapped, or reorganized before rendering.

The layout engine accounts for:

- Terminal columns
- Terminal rows
- Prompt height
- Input buffer height
- Completion menu height
- Right prompt width
- Wide Unicode characters
- Combining characters
- Escape sequence width
- Cursor position
- Selection spans
- Autosuggestion spans
- Tooltip placement
- Minimum viable segment widths
- Segment priority

Responsive behavior is deterministic. Important prompt content remains visible first. Less important segments are compressed, abbreviated, or removed according to priority and width rules.

Resize handling triggers immediate recomputation of all prompt, input, cursor, and completion layout.

## 15. Editor Engine

Keel includes a custom command-line editor. The editor owns the input buffer and all editing behavior while zsh is waiting for a command.

The editor supports:

- Text insertion
- Deletion
- Word movement
- Line movement
- Multiline buffers
- Selection
- Clipboard operations
- Undo and redo
- Bracket matching
- Quote-aware movement
- Syntax-aware operations
- Autosuggestion acceptance
- Completion insertion
- Mouse-based cursor positioning
- Mouse-based selection

The editor models the command buffer as structured editable text rather than raw terminal output. This allows rendering, cursor movement, syntax highlighting, and completion replacement to operate consistently.

## 16. Syntax Awareness

Keel parses enough shell syntax to support editing, highlighting, completion context, and safe replacement ranges. It does not execute shell syntax.

Syntax awareness includes:

- Command position
- Subcommand position
- Argument position
- Flag position
- Flag value position
- Quoted strings
- Escaped characters
- Environment assignments
- Pipelines
- Redirections
- Command substitutions
- Variable references
- Paths
- Globs

The parser provides context to the completion engine, syntax highlighter, autosuggestion engine, and cursor movement logic.

## 17. Completion System

Completion is a core feature of Keel. The system separates completion knowledge from completion presentation.

Zsh provides shell-specific completion knowledge. Keel owns completion UI, filtering, ranking, grouping, interaction, preview, and insertion.

The completion system receives:

- Buffer text
- Cursor position
- Parsed command context
- Current working directory
- Environment state
- Shell state
- History state
- Project context

The completion system returns candidates with:

- Display label
- Text to insert
- Description
- Completion kind
- Group
- Replacement range
- Priority
- Metadata
- Preview data where available

Keel renders completion candidates in its own UI. It supports fuzzy filtering, keyboard navigation, mouse interaction, grouped results, descriptions, previews, and stable replacement behavior.

## 18. zsh Completion Bridge

Keel integrates with zsh completion through a bridge layer. The bridge asks zsh for completion candidates using zsh’s own completion knowledge, then adapts those candidates into Keel’s completion model.

The bridge provides access to:

- Commands
- Aliases
- Functions
- Builtins
- Files
- Directories
- Variables
- Options
- Command-specific completions
- User-defined completion functions
- Completion descriptions where available
- Replacement context where available

Keel may also provide native completion sources for common high-value contexts such as files, directories, git branches, recent directories, command history, package scripts, make targets, cargo tasks, and environment variables. These sources are merged with zsh-provided candidates and ranked by Keel.

## 19. History System

Keel uses zsh history as the source of shell history truth while maintaining its own searchable index for fast interaction.

The history system supports:

- Fuzzy search
- Prefix search
- Current-directory-aware ranking
- Frequency ranking
- Recency ranking
- Failed-command awareness
- Session-local history
- Editable history insertion
- Multiline command display
- Deduplication

History search returns commands into the editor buffer. The user can edit before submission.

## 20. Autosuggestions

Autosuggestions are generated from history, context, and completion sources. They are rendered inline as part of the editor frame.

Autosuggestion ranking considers:

- Current buffer prefix
- Current directory
- Recent commands
- Frequently used commands
- Command success history
- Project context

Autosuggestions are accepted through editor actions and integrated with cursor movement, selection, syntax highlighting, and completion state.

## 21. Mouse Interaction

Keel captures mouse input while in interactive editing modes. Mouse behavior is part of the editor and completion runtime.

Mouse interaction supports:

- Moving the cursor within the buffer
- Selecting text
- Clicking completion candidates
- Scrolling completion menus
- Interacting with history UI
- Interacting with prompt widgets where applicable
- Releasing mouse ownership during command execution

Mouse capture is mode-dependent. Keel owns mouse interaction during editing and releases or limits it during command execution and external program control.

## 22. Terminal Driver

The terminal driver abstracts terminal input and output. It handles keyboard input, mouse input, resize events, focus events, bracketed paste, cursor visibility, terminal capabilities, and output flushing.

The terminal driver supports:

- Raw input handling during editing
- Key decoding
- Modifier-aware keyboard input
- Mouse protocol handling
- Bracketed paste
- Focus events
- Resize events
- Truecolor output
- Alternate screen awareness where relevant
- Native cursor hiding and restoration
- Terminal state recovery

Terminal output is produced through frame diffs rather than uncontrolled printing.

## 23. Shell Lifecycle Integration

Keel coordinates with zsh across the interactive lifecycle.

The lifecycle is:

- Zsh becomes ready for an interactive command.
- Keel starts the editor runtime.
- Keel renders the active prompt and input surface.
- The user edits the command.
- Keel resolves completions, suggestions, history, and prompt updates as needed.
- The user accepts the command.
- Keel renders the final transient prompt.
- Keel returns the command to zsh.
- Zsh parses and executes the command.
- Zsh reports status, duration, directory changes, and shell state.
- Keel resumes at the next prompt.

This lifecycle keeps zsh in control of shell execution while keeping Keel in control of the interactive surface.

## 24. Async State and Prompt Widgets

Keel supports async state updates for prompt widgets and completion sources.

Async state may include:

- Git status
- Repository root
- Runtime versions
- Project metadata
- Directory information
- Completion candidates
- Clock or timer widgets
- Long-running prompt segments

Async tasks update runtime state. The frame scheduler decides when those updates are visible. Slow state is cached and refreshed without blocking the editor.

## 25. Caching

Keel maintains caches for expensive or frequently reused state.

Cacheable state includes:

- Git repository status
- Repository root paths
- Runtime detection
- Project metadata
- History index
- Recent directories
- Completion metadata
- Command descriptions

Caches are invalidated by directory changes, file system changes where available, command execution events, time-based expiration, and explicit user actions.

The prompt renderer can use cached state immediately and update visible state when async refreshes complete.

## 26. Command Execution Handoff

When a command is accepted, Keel finalizes the visual prompt state and passes the command to zsh for normal execution.

During command execution:

- Keel restores terminal state as needed.
- The submitted command is handled by zsh.
- Interactive programs receive normal terminal behavior.
- Command output flows normally.
- Job control remains under zsh.
- Keel does not render prompt UI over command output.

After command completion, zsh provides status and lifecycle information. Keel uses that information to render the next prompt.

## 27. Reliability and Recovery

Keel preserves terminal usability even when failures occur.

Reliability requirements include:

- Restoring the native cursor on failure
- Restoring terminal input mode on failure
- Avoiding Rust panics crossing the zsh boundary
- Handling resize during editing
- Handling interrupted rendering
- Handling incomplete async updates
- Handling invalid completion responses
- Handling terminal capability differences
- Providing a recovery path back to normal zsh behavior

The native module isolates unsafe zsh integration from the Rust runtime and enforces clear ownership of memory, terminal state, and failure boundaries.

## 28. Configuration Model

Keel is opinionated but allows configuration of major product-level behavior.

Configuration may include:

- Prompt layout style
- Color palette
- Cursor style
- Animation intensity
- Frame-rate ceiling
- Completion behavior
- History ranking behavior
- Keymap preset
- Mouse support
- Transient prompt style
- Runtime feature toggles

Configuration is product-level rather than per-detail. The design system remains owned by Keel.

## 29. Debugging and Diagnostics

Keel includes diagnostic tools for installation, module loading, terminal capability detection, rendering state, completion bridge behavior, and shell lifecycle integration.

Diagnostics report:

- zsh version
- Module load status
- Terminal type
- Color support
- Keyboard protocol support
- Mouse support
- Prompt lifecycle status
- Completion bridge status
- Cache status
- Render timing
- Frame rate behavior
- Recent runtime errors
- Native module boundary errors

This is essential because Keel depends on native shell integration, terminal behavior, and user-specific zsh configuration.

## 30. Implementation Phases

### Phase 1: Native Module Skeleton

Establish the zsh module, load and unload behavior, basic activation, Rust FFI boundary, terminal state setup, and safe failure handling.

The goal is to prove that zsh can invoke Keel’s Rust runtime during interactive command reading and receive an accepted command back for execution.

### Phase 2: Minimal Custom Editor

Implement the basic editor runtime with text insertion, deletion, cursor movement, command acceptance, frame rendering, terminal cursor hiding, terminal restoration, and resize handling.

The goal is to prove that Keel can own the command-line surface without corrupting terminal output.

### Phase 3: Frame Scheduler and Cursor Animation

Add time-based rendering, frame diffing, animated cursor movement, blink fade simulation, prompt repainting, and transient prompt rendering.

The goal is to prove that Keel can provide a high-quality rendered prompt/editor experience rather than a hook-driven shell prompt.

### Phase 4: Prompt Runtime

Build the semantic prompt model, prompt segments, right prompt, async widgets, command duration, exit status, current directory display, git context, and responsive truncation.

The goal is to make the prompt a native part of Keel’s rendering system.

### Phase 5: History and Autosuggestions

Add zsh history indexing, fuzzy history search, inline autosuggestions, history ranking, and editable command insertion.

The goal is to make everyday editing faster and more fluid.

### Phase 6: Completion Engine

Build Keel’s completion model, candidate UI, fuzzy filtering, grouping, preview behavior, insertion logic, and replacement ranges.

The goal is to make completion feel native to Keel’s rendered interface.

### Phase 7: zsh Completion Bridge

Integrate zsh completion knowledge into Keel’s completion engine. Adapt zsh candidates into Keel’s completion item model and merge them with native Keel sources.

The goal is to preserve zsh’s completion power while replacing its presentation and interaction model.

### Phase 8: Mouse and Selection

Add mouse capture, cursor placement, selection, completion interaction, scroll behavior, and prompt widget interaction.

The goal is to make the command line feel like a modern editable surface.

### Phase 9: Hardening and Compatibility

Improve terminal compatibility, shell integration reliability, error recovery, module stability, diagnostics, cache invalidation, and user configuration.

The goal is to turn the prototype into a dependable daily shell layer.

## 31. Repository Structure

The repository is organized around the runtime boundary rather than shell scripts. Keel should be a Rust workspace with a small native zsh module boundary and a set of focused crates for the editor, renderer, prompt system, completion system, terminal driver, history engine, shell model, diagnostics, and CLI tooling.

The structure should keep unsafe shell integration separate from the reusable runtime. Rendering, editing, completion, history, and prompt behavior should be testable without loading zsh. The native module should bridge zsh into the Rust runtime rather than containing product logic.

Recommended top-level areas:

- `crates/` for the Rust workspace
- `zsh-module/` for the native zsh module and C or C-compatible ABI boundary
- `shell/` for minimal zsh setup, completions, install snippets, and fallback helpers
- `docs/` for architecture, rendering, completion, zsh integration, terminal behavior, and roadmap documentation
- `tests/` for integration tests, fixtures, terminal-frame snapshots, and replay tests
- `examples/` for sample zsh configurations, layouts, fixtures, and demonstration sessions
- `assets/` for palettes, glyph fallback metadata, and built-in visual resources
- `xtask/` for development automation, release scripts, packaging, and test harness commands

### 31.1 Rust Crate Layout

The Rust workspace should be split by domain rather than by implementation convenience. Each crate should have a narrow reason to exist and should avoid circular ownership of editor, renderer, shell, and completion responsibilities.

Recommended crates:

- `keel-core`: shared application types, configuration, events, modes, identifiers, errors, timing primitives, and common state models.
- `keel-editor`: command buffer, cursor movement, selection, undo and redo, clipboard ring, text motions, grapheme-aware editing, shallow shell syntax lexing, and editor actions.
- `keel-render`: virtual frame representation, terminal cell model, layout engine, frame diffing, terminal patches, style system, width calculation, cursor animation, prompt widgets, completion menu widgets, tooltips, and frame scheduling.
- `keel-complete`: completion request and response model, context detection, shell-aware replacement ranges, insertion edits, candidate grouping, fuzzy ranking, filtering, source registry, and completion menu state.
- `keel-prompt`: semantic prompt model, prompt state, active prompt, right prompt, transient prompt, prompt segment definitions, async prompt state, cache invalidation, and prompt layout hints.
- `keel-terminal`: terminal input and output normalization, key events, mouse events, paste events, focus events, resize events, raw mode, truecolor output, native cursor visibility, screen control, and terminal capability detection.
- `keel-history`: zsh history parsing, history entries, persistent index, fuzzy querying, cwd-aware ranking, recency ranking, frequency ranking, session history, and success/failure metadata.
- `keel-shell`: shell-agnostic context model for cwd, environment, aliases, functions, jobs, command lifecycle, runtime detection, path handling, Git context, and shell status.
- `keel-zsh-bridge`: safe Rust-facing wrappers around zsh state, zsh parameters, zsh completion bridge behavior, command acceptance, module lifecycle, and panic/error boundaries.
- `keel-cli`: developer and user-facing command-line tooling such as doctor, version, debug, inspect, replay, cache management, and diagnostics.
- `keel-devtools`: optional debugging tools such as frame inspection, event replay, render timing overlays, completion inspection, and state dump utilities.

Early prototypes may begin with fewer crates, but the target architecture should preserve these boundaries. A useful initial workspace is `keel-core`, `keel-editor`, `keel-render`, `keel-complete`, `keel-terminal`, and `keel-cli`, with prompt, history, shell, and zsh bridge crates split out as the prototype stabilizes.

### 31.2 Internal Crate Organization

Each crate should be organized around domain objects and state transitions rather than generic utility modules. Broad `utils` modules should be avoided unless they contain genuinely shared implementation-neutral helpers.

`keel-core` should contain:

- Application state types
- Configuration models
- Runtime events
- Accepted command metadata
- Mode definitions
- Stable identifiers for frames, requests, completions, and sessions
- Timing and clock primitives
- Shared error types
- A small prelude for common internal types

`keel-editor` should contain:

- Main editor state
- Rope-backed buffer model
- Line and column calculations
- Grapheme-aware indexing
- Buffer snapshots
- Logical cursor position
- Visual cursor mapping
- Cursor movement commands
- Selection ranges and selection actions
- Undo and redo stacks
- Edit transactions
- Clipboard ring
- Input actions and keymap definitions
- Shell lexer tokens
- Syntax highlighting metadata
- Unit tests for movement, editing, Unicode handling, and text actions

`keel-render` should contain:

- Renderer entry point
- Virtual frame model
- Terminal cell model
- Frame diff model
- Terminal patch model
- Layout engine
- Layout constraints
- Controlled wrapping
- Truncation rules
- Viewport calculations
- Color and style model
- Palette model
- Fake-opacity blending for cursor and animation effects
- Cursor animation state
- Cursor blink and fade curves
- Cursor shape rendering
- Prompt, buffer, completion menu, tooltip, scrollbar, and status widgets
- Frame clock and dirty-frame scheduling
- Snapshot tests for wrapping, frame diffing, width calculation, and visual output

`keel-complete` should contain:

- Completion engine
- Completion request model
- Completion response model
- Completion item model
- Replacement range and insertion edit model
- Completion context detector
- Shell-word and shell-position analysis
- Flag, argument, file, redirect, variable, pipe, and quoted-string contexts
- Candidate ranking
- Fuzzy matching integration
- Recency, frequency, and cwd-aware ranking
- Completion source trait or equivalent source abstraction
- Native sources for files, directories, commands, aliases, functions, environment variables, history, Git, npm, Cargo, Make, Docker, SSH, and zsh-backed completions
- Completion menu state, grouping, preview data, scrolling, selection, and actions
- Tests for context detection, replacement edits, ranking, and source behavior

`keel-prompt` should contain:

- Active prompt model
- Right prompt model
- Transient prompt model
- Prompt segment model
- Prompt state model
- Prompt-specific layout hints
- Async prompt state refresh
- Prompt cache storage
- Prompt cache invalidation
- Segments for cwd, Git, status, duration, jobs, runtime, virtual environment, and time
- Tests for prompt layout, transient prompt behavior, and segment state

`keel-terminal` should contain:

- Terminal handle abstraction
- Backend adapters for Crossterm, termwiz, or another terminal backend
- Normalized input event model
- Key event model
- Mouse event model
- Bracketed paste model
- Focus event model
- Resize event model
- Output writer
- ANSI and terminal-control helpers
- Native cursor hide/show handling
- Screen clearing and screen-region behavior
- Terminal capability detection
- Raw-mode ownership
- Tests for input decoding, terminal patches, and escape handling

`keel-history` should contain:

- History entry model
- History store abstraction
- zsh extended history parser
- Plain history parser
- Persistent search index
- History query model
- Recency ranking
- Frequency ranking
- Cwd-aware ranking
- Success/failure ranking
- Current session history
- Tests for zsh history parsing, ranking, and search

`keel-shell` should contain:

- Shell context model
- Environment model
- Alias model
- Function model
- Job model
- Command lifecycle model
- Runtime detection model
- Path model
- Git context model
- Tests for environment, path, and shell context behavior

`keel-zsh-bridge` should contain:

- Raw zsh FFI declarations
- FFI type definitions
- Safety wrappers around raw zsh access
- Module lifecycle abstractions
- Builtin registration abstractions
- Widget or editor-entry abstractions
- zsh parameter access
- zsh environment access
- zsh cwd and status access
- zsh jobs access where feasible
- zsh completion bridge request and response conversion
- Accepted-line handoff behavior
- Rust panic boundary handling
- zsh-facing error model

`keel-cli` should contain:

- CLI argument parsing
- Doctor command
- Version command
- Debug command
- Inspect command
- Replay command
- Cache command
- Human-readable output formatting
- Diagnostics output formatting

### 31.3 Native zsh Module Layout

The native module should be intentionally small and should remain focused on integration. It should not implement editor behavior, rendering behavior, completion ranking, prompt layout, or terminal UI. Its purpose is to load into zsh, expose zsh state, connect zsh lifecycle events to Rust, call the Rust editor runtime, and safely return accepted commands to zsh.

Recommended module areas:

- Module initialization and teardown
- Builtins such as enable, disable, doctor, and debug entry points
- Editor entry point that invokes the Rust runtime
- Completion bridge hooks
- zsh state extraction
- Terminal handoff details
- zsh allocation and memory helpers
- Compatibility helpers for supported zsh versions
- Rust bridge headers
- Error and panic boundary handling

The module should be treated as a high-risk boundary. It should be small, defensive, well-tested through integration harnesses, and designed so that failures can restore terminal state whenever possible.

### 31.4 Shell Integration Layout

The shell integration directory should remain minimal. It should provide loading snippets, user-facing enable/disable helpers, shell completions for Keel’s own CLI, installation snippets, and uninstall helpers. It should not contain core product behavior.

Recommended shell areas:

- Main `keel.zsh` loader
- Completion definition for Keel’s own CLI
- Enable and disable helpers
- Doctor helper
- zshrc installation snippet
- Uninstall helper

### 31.5 Test and Fixture Layout

Keel should include tests for pure Rust systems, native zsh integration, and terminal rendering behavior. Terminal UI bugs are difficult to reason about manually, so replay-based and snapshot-based tests should be part of the architecture from the beginning.

Recommended test areas:

- Editor loop integration tests
- Basic command execution tests
- Completion flow tests
- Resize behavior tests
- Transient prompt tests
- zsh history fixtures
- Completion fixtures
- Git repository fixtures
- Terminal-frame fixtures
- zsh configuration fixtures
- Render snapshots for prompt, completion menu, cursor states, wrapping, and terminal diffing
- Recorded session replays with input events, terminal sizes, shell state, and expected frames

The replay system should allow recorded events and shell state to be rendered deterministically so cursor movement, resizing, completion menu behavior, wrapping, and transient prompt output can be tested without relying on manual terminal sessions.

### 31.6 Dependency Direction

The dependency graph should preserve the boundary between editor state, rendering, completion, terminal handling, and shell integration. The editor should not depend on the renderer. The renderer may consume editor snapshots. Completion may consume editor snapshots and shell context. The zsh bridge may feed shell state into the runtime. Core systems should not directly call zsh.

Preferred dependency principles:

- `keel-core` may be used by all crates.
- `keel-editor` owns editing behavior and should remain independent of rendering and zsh.
- `keel-render` consumes semantic state and produces frames.
- `keel-complete` consumes editor snapshots and shell context and produces completion edits.
- `keel-prompt` produces semantic prompt state and does not emit terminal patches directly.
- `keel-terminal` owns terminal input/output details and should not contain product logic.
- `keel-history` owns history parsing, indexing, and ranking.
- `keel-shell` models shell state without binding itself to zsh internals.
- `keel-zsh-bridge` is the only Rust crate that understands zsh-specific internals.
- `zsh-module` is the only native module boundary and should delegate behavior to Rust.

This structure allows Keel to evolve without turning the runtime into one large, shell-specific implementation unit.

## 32. Success Criteria

Keel is successful when the user experiences zsh through a faster, smoother, more capable command-line surface without losing zsh behavior.

Primary success criteria:

- The editor feels immediate.
- Cursor animation feels intentional and smooth.
- Prompt updates are stable and frame-based.
- Terminal resizing is clean.
- Transient prompts preserve readable scrollback.
- Completion feels richer than default zsh completion.
- History search is fast and useful.
- Command execution behaves like normal zsh.
- Terminal state recovers reliably after errors.
- Daily use does not require thinking about the integration layer.

## 33. Core Principle

Keel replaces the part of zsh that the user directly touches while preserving the shell behavior that makes zsh powerful. It is a rendered command-line interface layer backed by zsh, built around a native module, a Rust runtime, a frame scheduler, a custom editor, and a completion system that combines Keel’s interface with zsh’s shell knowledge.
