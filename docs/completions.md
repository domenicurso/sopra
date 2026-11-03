# Completion architecture

Sopra builds completion data from the command the user is currently editing. The parser probes help forms and manual pages, normalizes each source into layout rows, and merges the results into one graph. The editor initially loads the root and its immediate child metadata, then hydrates deeper command paths only when the cursor reaches them, so recursive coverage does not make the first completion request wait for every descendant.

The completion subsystem has four boundaries:

- `parser` owns process discovery, source caching, help/man normalization, and incremental parsing of nested command paths.
- `graph` owns the command, option, positional, value, and alias model produced by parsers.
- `engine` owns shell-aware context resolution, including pending option values, paths, variables, and the node selected by the current token sequence.
- `ranking` and the editor own presentation order, fuzzy matching, highlighting, and the completion response sent back to Zsh.

## Command-agnostic invariant

Production completion logic must not contain hardcoded behavior for one command, subcommand, option, output phrase, executable path, or vendor. A command name may appear in a fixture or a manually inspected corpus as representative input, but it must never select a special branch in the parser or engine.

When a tool exposes an unusual format, add a generic rule for the format invariant—for example, a POSIX option cluster, a GNU table, an argparse usage form, a Clap section, a Cobra command row, an mdoc layout, or a wrapped man-page paragraph. Add a focused fixture that proves the rule, then check it against real output from more than one tool that uses the same convention. If the proposed fix starts with `program == "..."`, stop and identify the missing grammar or source boundary instead.

## Source aggregation

Help, man pages, and future first-party completion sources are evidence for the same graph rather than competing backends. Each source should retain its provenance while parsing, so a stronger source can fill a missing description or nested node without erasing useful data from another source. The source cache is keyed by executable identity, requested command path, working directory, executable metadata, and the standard environment variables that affect command lookup, configuration paths, and manual rendering; dynamic providers remain an extension point, and dynamic results such as repository refs must remain runtime candidates rather than static graph entries. Help summaries take precedence over manual paragraphs, while manuals can add missing options, values, descriptions, and commands.

The graph is one logical object even though its source fragments are cached by path. A root request stores the command and its immediate children, while a request for `npm access` merges that child fragment into the existing root graph instead of creating a separate graph. `sopra debug completions --root npm` prints the first chunk, `sopra debug completions --chunk npm access` prints a nested chunk, and `sopra debug completions npm` performs the explicit full recursive audit.

## Validation

Parser changes need both focused fixtures and a manual real-output pass. The manual pass reads the source output and checks command names, aliases, options, value placeholders, descriptions, recursive subcommands, and exclusions such as prose or examples being mistaken for candidates. The corpus should span POSIX, GNU, argparse, Clap, Cobra, mdoc/BSD, man-rendered, table-oriented, and bespoke help layouts; a passing parser test alone does not establish that a candidate list matches the command's published interface.

## Cache lifecycle and presentation

The running engine keys graphs with the same executable, directory, and environment identity as source discovery, so rebuilding an executable or changing directories starts a new discovery context. A graph entry owns its root, hydrated command paths, and load time together. Positive and negative entries expire after 15 minutes, and the engine retains at most 128 roots. Expiration and eviction remove descendant hydration with the root, so stale child state cannot prevent a new probe. Disk bundles use a bounded binary container with length-framed help/man records, duplicate elimination, and an integrity checksum. Unknown record kinds can be skipped. The cache key includes a build-time fingerprint of the collector and codec, so their changes invalidate old bundles automatically without manually incrementing a V7/V8 schema constant. Parser-only changes reparse the stored sources. Bundles contain raw sources, expire after 15 minutes, and are replaced atomically; parsers always run against the cached sources rather than reusing serialized graphs.

The graph retains shell argument boundaries. Presentation collapses a chain of single-child grouping nodes into one suggestion, such as `widgets list`, only while intermediate nodes have no options or positionals. Branching groups remain separate suggestions, and a user who types a group explicitly can still complete its children.

## Repeatable source and terminal audits

Run `npm --prefix scripts run test:completions` after building the worker. The terminal test drives an isolated Zsh through nested command acceptance, multiword suggestions, option choices, executable replacement, and changing directories. Its fixture executables and configuration live in temporary directories.

The graph debugger test verifies offline help/man parsing, source reuse, corruption recovery, and explicit cache refresh through the public CLI.

The live audit (`python3 scripts/completion-audit.py`) independently extracts command inventories and declared flags from installed CLI help and compares them with root and nested graphs, including a second read through the warm cache. Its corpus covers Sopra, Git, Cargo, rustup, ripgrep, bat, GitHub CLI, npm, Bun, Deno, pnpm, javac (including advertised extended help), Java, jar, curl, tar, and Python. Uninstalled tools are reported as skipped. Help and graph evidence is saved outside the repository; `--output /path/to/new-directory` selects a retained evidence directory. This checks the advertised static interface, not every runtime value or undocumented command.

Use focused Rust regression cases for format rules so correctness does not depend on these tools being installed. Live inventory assertions supplement those cases and the manual comparison of descriptions, aliases, placeholders, and exclusions; they do not prove that arbitrary future CLI layouts will parse correctly.

## Graph inspection

`debug completions` remains available; `debug graph` exposes the source and persistence boundaries explicitly:

```sh
sopra debug graph generate --fresh --root javac
sopra debug graph generate --chunk sopra dev
sopra debug graph sources --root javac
sopra debug graph cache --root javac
sopra debug graph parse example /tmp/example-help.txt
cat /tmp/example-help.txt | sopra debug graph parse example -
sopra debug graph parse --man example /tmp/example.1
```

Generation probes an installed command; omitting `--root` or `--chunk` requests recursive discovery. `--fresh` bypasses and replaces cached sources. `sources` prints the collected evidence and whether it came from disk, while `cache` reports the bundle path, age, integrity, and source sizes. `parse` reads captured help or manual text without resolving or executing the named command. These commands share the editor's parser, so captured evidence can reproduce a graph defect without installing its originating CLI.
