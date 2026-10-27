# External dependency metadata

Semantic analysis can attach direct package metadata to Rust, Python, JavaScript,
and TypeScript type references. This helps an agent choose documentation and APIs
for the repository's dependency requirements. It does not download package source
or invoke a package manager.

```bash
context-creator src --include-types --max-tokens 8000
context-creator context --seed src/lib.rs --task "Review serialization"
```

Markdown, plain text, and XML output include external package annotations. Agent
context reports `external_type` gaps containing the same metadata, because the
external implementation is unavailable. These gaps count toward the response's
token budget. The crate exposes `TypeReference.external_package` and the
`core::semantic::packages::PackageVersionResolver` implementations; both MCP
servers use the shared context service.

## Requirements and versions

| Metadata | Meaning |
| --- | --- |
| `serde (declared: ^1.0)` | A direct dependency requirement; no unique compatible lockfile version was found. |
| `serde v1.0.197 (locked; declared: ^1.0)` | One compatible version appears in the repository lockfile. This is evidence from the lockfile, not verification of the installed environment. |
| `django (declared: ==3.2.0)` | An exact pin in a declaration; the environment has not been inspected. |
| `react (declared: ^18.0.0; dev)` | A dependency found only in a development section. |
| `serde` | External package name only; version metadata was unavailable. |

Rust reads the nearest package's Cargo dependency sections, including target,
build, renamed, and inherited workspace dependencies. `Cargo.lock` supplies a
version only when compatible candidates agree; ambiguous duplicate versions keep
the declared requirement.

JavaScript/TypeScript read the nearest `package.json`, including development,
optional, and peer dependencies and npm aliases. npm `package-lock.json` v1–v3
can supply compatible versions, including nested member installations. Yarn/pnpm
lockfiles, registry tags, and complex npm range syntax may leave only a declared
requirement. Workspace and local path packages use source relationships instead.

Python reads `requirements.txt`, `requirements-dev.txt`, PEP 621 project and
optional dependencies, Poetry dependency/group tables, and literal `setup.py`
metadata. Environment markers are not evaluated; declarations remain requirements.
Requirements-file includes, computed setup arguments, Python lockfiles, and import
to distribution renames such as `PIL` → `Pillow` are unavailable. Missing metadata
is preferable to guessing that mapping.

## Boundaries and freshness

Only direct declarations receive versions. A package appearing solely in a
lockfile does not establish a direct dependency. Analyzers include development
metadata and mark it `dev`; library callers can pass `include_dev = false` to a
package resolver to exclude development-only declarations.

The shared manifest cache checks content, including same-size edits, and reuses
parsed documents. Reads are bounded to 1 MiB and stay inside the explicit project
root with hidden/symlink restrictions. Invalid or missing metadata leaves analysis
usable. A malformed member manifest does not borrow unrelated ancestor declarations.
Credential-bearing URLs and arbitrary manifest strings are never copied
into version annotations. No package source is fetched or executed.

See [workspace resolution](workspaces.md) and [agent context](agent-tools.md).
