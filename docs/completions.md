# Completion architecture

Sopra builds completion data from the command the user is currently editing. The parser probes help forms and manual pages, normalizes each source into layout rows, and merges the results into one graph. The editor initially loads the root and its immediate child metadata, then hydrates deeper command paths only when the cursor reaches them, so recursive coverage does not make the first completion request wait for every descendant.

The completion subsystem has five boundaries:

- `parser` owns process discovery, source caching, help/man normalization, and incremental parsing of nested command paths.
- `graph` owns the command, option, positional, value, and alias model produced by parsers.
- `engine` owns shell-aware context resolution, including pending option values, paths, variables, and the node selected by the current token sequence.
- `providers` owns declarative runtime value queries, cancellation, bounded execution, and their short-lived cache.
- `ranking` and the editor own presentation order, fuzzy matching, highlighting, and the completion response sent back to Zsh.

## Command-agnostic invariant

Production completion logic must not contain hardcoded behavior for one command, subcommand, option, output phrase, executable path, or vendor. A command name may appear in a fixture or a manually inspected corpus as representative input, but it must never select a special branch in the parser or engine.

When a tool exposes an unusual format, add a generic rule for the format invariant—for example, a POSIX option cluster, a GNU table, an argparse usage form, a Clap section, a Cobra command row, an mdoc layout, or a wrapped man-page paragraph. Add a focused fixture that proves the rule, then check it against real output from more than one tool that uses the same convention. If the proposed fix starts with `program == "..."`, stop and identify the missing grammar or source boundary instead.

## Source aggregation

Help, man pages, and future first-party completion sources are evidence for the same graph rather than competing backends. Each source should retain its provenance while parsing, so a stronger source can fill a missing description or nested node without erasing useful data from another source. The source cache is keyed by executable identity, requested command path, working directory, executable metadata, and the standard environment variables that affect command lookup, configuration paths, and manual rendering; dynamic results such as repository refs must remain runtime candidates rather than static graph entries. Help summaries take precedence over manual paragraphs, while manuals can add missing options, values, descriptions, and commands.

The graph is one logical object even though its source fragments are cached by path. A root request stores the command and its immediate children, while a request for `npm access` merges that child fragment into the existing root graph instead of creating a separate graph. `sopra debug completions --root npm` prints the first chunk, `sopra debug completions --chunk npm access` prints a nested chunk, and `sopra debug completions npm` performs the explicit full recursive audit.

## Validation

Parser changes need both focused fixtures and a manual real-output pass. The manual pass reads the source output and checks command names, aliases, options, value placeholders, descriptions, recursive subcommands, and exclusions such as prose or examples being mistaken for candidates. The corpus should span POSIX, GNU, argparse, Clap, Cobra, mdoc/BSD, man-rendered, table-oriented, and bespoke help layouts; a passing parser test alone does not establish that a candidate list matches the command's published interface.

## Cache lifecycle and presentation

The running engine keys graphs with the same executable, directory, and environment identity as source discovery, so rebuilding an executable or changing directories starts a new discovery context. A graph entry owns its root, hydrated command paths, and load time together. Negative entries expire after 15 minutes. Positive entries become stale after 15 minutes and remain available during refresh; a failed refresh retains the last good graph. The engine retains at most 128 roots, and eviction removes descendant hydration with the root. Disk bundles use a bounded binary container with length-framed help/man records, duplicate elimination, and an integrity checksum. Unknown record kinds can be skipped. The cache key includes a build-time fingerprint of the collector and codec, so their changes invalidate old bundles automatically without manually incrementing a V7/V8 schema constant. Parser-only changes reparse the stored sources. Bundles contain raw sources, expire after 15 minutes, and are replaced atomically; source inspection always reparses this evidence.

The graph distinguishes separate, equals-attached, colon-attached, and adjacent values, optional values, repeatable arguments, and explicit mutually exclusive choices in usage expressions. The engine stops offering options after `--`, excludes consumed nonrepeatable options and known conflicts, and preserves the argument replacement range during insertion. It does not infer undocumented constraints.

The graph retains shell argument boundaries. Presentation collapses a chain of single-child grouping nodes into one suggestion, such as `widgets list`, only while intermediate nodes have no options or positionals. Branching groups remain separate suggestions, and a user who types a group explicitly can still complete its children.

## Repeatable source and terminal audits

Run `npm --prefix scripts run test:completions` after building the worker. The terminal test drives an isolated Zsh through nested command acceptance, multiword suggestions, option choices, live values, executable replacement, and changing directories. Its fixture executables and configuration live in temporary directories.

The maintained contract fixture independently specifies descriptions, aliases, attachment syntax, optionality, repeatability, and choices. The contract test executes completed lines through stock Zsh and checks the resulting argument array, including Unicode, quoting, attached file paths, and a cursor in the middle of a line.

The graph debugger test verifies offline help/man parsing, source reuse, corruption recovery, and explicit cache refresh through the public CLI.

The live audit (`python3 scripts/completion-audit.py`) independently extracts command inventories and declared flags from installed CLI help and compares them with root and nested graphs, including a second read through the warm cache. Structured JSON checks also compare alias grouping and explicitly enumerated choices from declaration rows. Its corpus covers Sopra, Git, Cargo, rustup, ripgrep, bat, GitHub CLI, npm, Bun, Deno, pnpm, javac (including advertised extended help), Java, jar, curl, tar, and Python. Uninstalled tools are reported as skipped. Help and graph evidence is saved outside the repository; `--output /path/to/new-directory` selects a retained evidence directory. This checks the advertised static interface, not every runtime value or undocumented command.

Use focused Rust regression cases for format rules so correctness does not depend on these tools being installed. Live inventory assertions supplement those cases and the manual comparison of descriptions, aliases, placeholders, and exclusions; they do not prove that arbitrary future CLI layouts will parse correctly.

## Graph inspection

`debug completions` remains available; `debug graph` exposes the source and persistence boundaries explicitly:

```sh
sopra debug graph generate --fresh --root javac
sopra debug graph generate --chunk sopra dev
sopra debug graph sources --root javac
sopra debug graph cache --root javac
sopra debug graph explain --root javac -- --class-path
sopra debug graph json --chunk sopra dev
sopra debug candidates 'javac -g:v'
sopra debug providers
sopra debug graph parse example /tmp/example-help.txt
cat /tmp/example-help.txt | sopra debug graph parse example -
sopra debug graph parse --man example /tmp/example.1
```

Generation probes an installed command; omitting `--root` or `--chunk` requests recursive discovery. `--fresh` bypasses and replaces cached sources. `sources` prints the collected evidence and whether it came from disk, while `cache` reports the source bundle path, age, integrity, source sizes, and exact/provisional graph snapshot status. `parse` reads captured help or manual text without resolving or executing the named command. These commands share the editor's parser, so captured evidence can reproduce a graph defect without installing its originating CLI.


## Immediate suggestions and discovery scheduling

Derived graph snapshots use a checksummed Postcard binary payload with a two-megabyte per-file limit and atomic replacement. Background maintenance trims the store to roughly 512 recent files every 32 writes; individual readers reject files older than seven days. A build script fingerprints the completion sources and manifest automatically, retiring incompatible snapshots without a manually maintained schema version. Snapshots omit provenance text; raw source bundles remain the authority for explanation and reparsing.

A new worker restores an exact-context snapshot directly on the suggestion path. If none exists, it may use a provisional snapshot for the same executable and command path from another directory or executable build, then refresh it asynchronously. Provisional suggestions can briefly reflect the previous context. Snapshots older than seven days are rejected. The first-ever command still requires evidence collection; the frame displays available local candidates while that work runs. Prefix prefetch starts after two command characters, and only the best candidate is warmed.

Foreground discovery runs before queued prefetch and cancels an unrelated active probe. Promoting an already queued path preserves pending bookkeeping. Ending an edit cancels obsolete work. There are at most 64 queued discovery requests, six concurrent probe subprocesses across discovery and live providers, a 600ms per-process ceiling, and a two-second interactive discovery budget. Explicit recursive debugging has a 15-second budget, depth limit eight, and node limit 512. Timed-out or cancelled process groups are terminated, output is bounded to one megabyte, and cancelled fragments never replace a usable graph.

The snapshot regression test times the first local completion after a worker restart and directory change against a fixture executable that sleeps for five seconds; suggestions must arrive within a 16ms frame without executing it. This is an enforced fixture budget, not a guarantee for all disks, graph sizes, or machines.

## Live value recipes

Bundled recipes supply Git switch branches and checkout refs, Docker start/log container names, and npm run script names. They live in `crates/completion/src/providers/builtin.toml`; command-specific behavior stays in data rather than parser branches. Execution passes literal argument arrays to a subprocess without interpolating the typed input into shell commands. JSON recipes read object keys directly.

User recipes live at `$XDG_CONFIG_HOME/sopra/completions.toml` (default `~/.config/sopra/completions.toml`). Matching IDs replace bundled recipes, and `enabled = false` disables one. `sopra debug providers` validates the file and lists active recipes. For example:

```toml
[[provider]]
id = "project-environments"
command = "project-cli"
path = ["deploy"]
option = "--environment"
exec = ["project-cli", "environments", "--names-only"]
timeout_ms = 150
ttl_ms = 300
root_marker = ".project"
watch = [".project/environments.json"]
```

Specify exactly one `option` name or zero-based `positional` index. Instead of `exec`, a recipe can use `json_file = "package.json"` and `pointer = "/scripts"` to return object keys. `root_marker` finds the nearest ancestor containing that path; JSON and watch paths resolve there, while executable recipes run in the actual shell directory. Output is one candidate per line. Candidates retain their display names and are escaped as individual shell arguments for insertion.

Providers run on a separate worker, replace queued obsolete queries, and return only to the current edit generation. Their cache holds at most 64 entries, keys include recipe, executable/environment identity, working directory, and watched file metadata, and TTL defaults to 300ms (maximum five seconds). Timeouts default to 150ms (maximum 500ms); a request has a 500ms total budget. The short TTL also bounds staleness when a daemon or a Git worktree changes outside watched paths. Provider failures leave static completion available; `debug candidates` reports provider errors directly. File metadata invalidation does not detect a rewrite that preserves both size and modification time.

## Entry provenance

Graph aggregation retains observations for each option, positional, and child command before merging sources. `debug graph explain` prints the selected fields, matching normalized help or rendered-man source lines, each source's parsed observation, and the merge policy. Source numbering matches `debug graph sources`; rendered-man line numbers refer to converted text, not original roff bytes. Exact parser byte spans and the originating probe flag are not retained. A source without a direct declaration match is reported as inferred rather than assigned an invented location.
