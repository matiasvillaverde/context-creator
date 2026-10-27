# Workspace context

Local package imports can resolve to sibling source inside the repository root.
The crate, CLI, and MCP use the same bindings. Package managers, build scripts,
and `setup.py` are never executed to discover them.

For the ordinary CLI, select the member you want to read:

```bash
context-creator path/to/workspace/app/src --include-types \
  --max-tokens 8000 --secret-scan redact
```

The detected Git root bounds expansion. Without Git metadata, declared workspace
membership can identify the enclosing workspace. For task context, supply the
workspace as the explicit root and the member file as a seed:

```bash
context-creator context --root path/to/workspace --seed app/src/lib.rs \
  --max-tokens 8000 --secret-scan redact
```

An explicit root is a boundary: a dependency outside it stays unavailable even
when a manifest declares a path to it. Ignore rules, hidden-file filtering, and
symlink restrictions still apply. No external package source is downloaded.

## Supported manifest bindings

| Ecosystem | Source discovery |
| --- | --- |
| Rust | Cargo path dependencies, dependency aliases, inherited workspace dependencies, standard dependency sections, custom library paths, and local module/re-export chains |
| Python | Poetry path dependencies and groups; uv workspace/path sources; PEP 508 local file requirements in project dependencies; literal `setup()` metadata; conventional and configured source layouts |
| JavaScript / TypeScript | npm/Yarn workspace member lists and pnpm `packages` lists; local `file:`/`link:` dependencies; package entry points and literal conditional/subpath exports |

Member patterns support directory globs and exclusions. pnpm supports block lists
and quoted inline lists; YAML aliases and computed configuration are unavailable.
Static Python setup metadata supports literal strings, lists, tuples, and
dictionaries passed to `setup()` or `setuptools.setup()`. Computed arguments,
environment-dependent paths, arbitrary packaging hooks, and runtime `sys.path`
changes need additional information.

Type imports and aliases point to their original definition when that binding is
available. Re-export traversal retains depth and file-cycle guards. An unresolved
type in a known package does not trigger a search through unrelated packages.
Repeated occurrences start independent type-resolution chains and do not count
as circular references merely because their name was already seen.

Explicit Cargo paths and forced workspace bindings can be marked `resolved`.
Python search layouts, ordinary Node version-based workspace candidates, and
source-entry fallbacks are `inferred`. Supported Node ranges include exact
versions, common caret/tilde/comparison ranges, wildcards, and alternatives.
Incompatible versions and duplicate workspace names do not select a package.
These relationships describe static evidence rather than complete compiler,
installer, bundler, or runtime behavior.

## Freshness and limits

Manifest parsing is cached by content, so same-size edits invalidate parsed data.
Dependency declarations are cached separately from package bindings and checked
against current source content. Agent sessions rebuild relationships when a
visible manifest changes and report the new source revision.

Metadata reads are limited to 1 MiB; definition-source reads to 2 MiB. Member
discovery examines declared directories, skips dependency/build and hidden
directories, and is bounded to 256 members, 2,048 directories per expansion, and
eight recursive directory levels. Unsupported or over-limit layouts remain
unresolved. Standard agent graph, candidate, and token limits also apply.

See [agent context](agent-tools.md), [language support](languages.md), and
[configuration](configuration.md) for response gaps, confidence, and filters.
