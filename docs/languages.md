# Semantic language support

All registered analyzers use real tree-sitter grammars. Text ingestion supports
additional extensions; the table describes semantic extraction and local source
resolution, rather than syntax highlighting or compiler compatibility.

| Language family | Source extensions | Local resolution |
| --- | --- | --- |
| Rust | `rs` | Modules, crate paths, local source |
| Python | `py`, `pyi`, `pyw` | Relative imports and local modules |
| JavaScript | `js`, `jsx`, `mjs`, `cjs` | Relative modules and index files |
| TypeScript | `ts`, `tsx`, `mts`, `cts` | Relative modules and index files; TSX uses its own grammar |
| Go | `go` | Local module/package paths |
| Swift | `swift` | Local source/type discovery |
| Java | `java` | Packages, explicit/wildcard/static imports, same-package types; Maven/Gradle roots |
| C | `c`, `h` | Quoted includes, local/include/src directories, declarations and prototypes |
| C++ | `cpp`, `cc`, `cxx`, `hpp`, `hh`, `hxx`, `h` | Local headers and type declarations; C++ constructs in `.h` select the C++ analyzer |
| C# | `cs` | Local namespace/type files and using aliases |
| Kotlin | `kt`, `kts` | Package/type/function files, aliases, referenced wildcard-imported types, standard source roots |
| Ruby | `rb` | Literal `require` and `require_relative`, local/lib paths |
| PHP | `php` | Namespace imports/aliases, literal includes, Composer PSR-4 directories |

Use `--trace-imports`, `--include-callers`, and `--include-types` for expansion.
`--style map` extracts declarations, signatures, paths, and one-based line
numbers. Both forms preserve deterministic output ordering and project-root
boundaries. Parsing recovers useful declarations from malformed source and has
a cooperative 30-second deadline per parse. Async AST work runs on blocking
workers rather than the Tokio runtime.

Resolution is static and local. It does not execute preprocessors, compiler
plugins, Ruby/PHP runtime loads, package downloads, or application code. External
libraries, compiler classpaths, overload dispatch, macro-generated declarations,
and dynamic imports can remain unresolved. Caller matching is an approximation,
especially for common method names and dynamic dispatch. Fixtures exercise
extraction, imports, callers/importing clients, types, aliases, malformed syntax,
and symlink confinement; they do not prove compiler-level equivalence.
