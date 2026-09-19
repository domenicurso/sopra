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

Help, man pages, and future first-party completion sources are evidence for the same graph rather than competing backends. Each source should retain its provenance while parsing, so a stronger source can fill a missing description or nested node without erasing useful data from another source. The current cache is keyed by executable identity, requested command path, working directory, and executable metadata; source fingerprints and dynamic providers are extension points for the next aggregation pass, and dynamic results such as repository refs must remain runtime candidates rather than static graph entries.

The graph is one logical object even though its source fragments are cached by path. A root request stores the command and its immediate children, while a request for `npm access` merges that child fragment into the existing root graph instead of creating a separate graph. `completion-graph --root npm` prints the first chunk, `completion-graph --chunk npm access` prints a nested chunk, and `completion-graph npm` performs the explicit full recursive audit.

## Validation

Parser changes need both focused fixtures and a manual real-output pass. The manual pass reads the source output and checks command names, aliases, options, value placeholders, descriptions, recursive subcommands, and exclusions such as prose or examples being mistaken for candidates. The corpus should span POSIX, GNU, argparse, Clap, Cobra, mdoc/BSD, man-rendered, table-oriented, and bespoke help layouts; a passing parser test alone does not establish that a candidate list matches the command's published interface.
