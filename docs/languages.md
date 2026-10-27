# Semantic language support

All 29 registered language families use real tree-sitter grammars. This count
includes query, infrastructure, and markup languages. Text ingestion supports
additional extensions; the table describes semantic extraction and local source
resolution, rather than compiler compatibility. In agent context, direct local
bindings are `resolved`; search-path, namespace, and SQL candidates are `inferred`.

Feature coverage differs by language and syntax form. The
[open-source audit](oss-language-audit.md) sampled 87 files from 29 pinned public
projects and found unresolved import, declaration, caller/type, and performance
failures. Parser registration establishes availability; consult the audit before
assuming a feature works for a particular language.

[Workspace context](workspaces.md) follows declared Cargo, Python, and npm/Yarn/pnpm
local package bindings inside the selected repository boundary. It documents
supported layouts, entry points, confidence, and unresolved build/runtime cases.

| Language family | Source extensions | Local resolution |
| --- | --- | --- |
| Rust | `rs` | Modules, crate paths, local source |
| Python | `py`, `pyi`, `pyw` | Relative imports and local modules |
| JavaScript | `js`, `jsx`, `mjs`, `cjs` | Relative modules and index files |
| TypeScript | `ts`, `tsx`, `mts`, `cts` | Relative modules and index files; TSX uses its own grammar |
| Go | `go` | Local module/package paths, including every visible non-test source file in an imported package |
| Swift | `swift` | Local source/type discovery |
| Java | `java` | Packages, explicit/wildcard/static imports, same-package types; Maven/Gradle roots |
| C | `c`, `h` | Quoted includes, local/include/src directories, declarations and prototypes |
| C++ | `cpp`, `cc`, `cxx`, `hpp`, `hh`, `hxx`, `h` | Local headers and type declarations; C++ constructs in `.h` select the C++ analyzer |
| C# | `cs` | Local namespace/type files and using aliases |
| Kotlin | `kt`, `kts` | Package/type/function files, aliases, referenced wildcard-imported types, standard source roots |
| Ruby | `rb` | Literal `require` and `require_relative`, local/lib paths |
| PHP | `php` | Namespace imports/aliases, literal includes, Composer PSR-4 directories |
| Zig | `zig` | Literal file `@import` and `@embedFile`, import aliases, qualified/local types, container methods, and test declarations |
| Bash | `sh`, `bash` | Function declarations and literal `source`/`.` paths; runtime working directory is unverified |
| Dart | `dart` | Declarations, `import`/`export`/`part` paths, and the current package's `lib/` files via `pubspec.yaml` |
| Lua | `lua` | Functions, literal `require`/`dofile`/`loadfile`; module and `init.lua` search-path candidates are inferred |
| Scala | `scala`, `sc` | Functions, types, and conventional package/source paths; namespace binding is inferred |
| Elixir | `ex`, `exs` | Functions/modules, literal `Code.require_file`/`compile_file`, and inferred alias/import/require/use module paths |
| R | `r`, `R` | Function assignments and literal `source`/`sys.source`; library names remain external |
| Julia | `jl` | Long/short function definitions, modules/types, literal includes, and inferred module files |
| Haskell | `hs` | Functions/types and inferred module files under conventional local source roots |
| Clojure | `clj`, `cljs`, `cljc` | Function/type forms, namespace `:require`, and literal `load-file`; namespace paths are inferred |
| PowerShell | `ps1`, `psm1`, `psd1` | Functions, dot-source, and literal `Import-Module`; runtime module/search-path binding is inferred |
| Perl | `pl`, `pm` | Subroutines, `use`, and literal `require`; module and runtime include-path binding is inferred |
| SQL | `sql` | Table/view/function declarations; the agent index links referenced objects to unique local DDL files with inferred confidence |
| HCL / Terraform | `hcl`, `tf`, `tfvars` | Blocks, literal local module/template/file paths, and sibling `.tf` files in a module |
| HTML | `html`, `htm` | Elements with IDs, script/stylesheet/module-preload paths, and imports from inline JavaScript modules |
| CSS | `css` | Selector rules and literal `@import` paths |

SQL resolution does not consult a database catalog or migration state. Duplicate
object declarations and case-distinct quoted names stay unresolved. Terraform
membership links use a representative sibling file, with line 1 as the location;
they describe shared module scope, not a literal import statement. HTML/CSS
root-relative web paths search the repository, `public/`, and `static/` and remain
inferred. Non-JavaScript inline scripts are ignored. Embedded/template syntax may
produce parse gaps rather than complete language extraction. Perl uses the maintained
[upstream parser](https://github.com/tree-sitter-perl/tree-sitter-perl).

Zig uses the [upstream tree-sitter grammar](https://github.com/tree-sitter-grammars/tree-sitter-zig).
Literal file imports resolve relative to their source and remain inside the
repository. ZON manifests are included as text. Named modules (`std`, `builtin`,
`root`, and build-provided names) remain external/build-provided gaps; build
scripts, compiler-generated sources, C translation, computed imports, and escaped
import paths are not evaluated. `build.zig` and `build.zig.zon` are included as
configuration when building context for a Zig seed. See Zig's
[import semantics](https://ziglang.org/documentation/master/#import) and
[build system](https://ziglang.org/learn/build-system/).

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

For a file and its forward dependency tree, use
`context-creator context --root . --seed path/to/file --max-tokens 12000`, or set
MCP `read_file.dependency_tokens`. Forward traversal is cycle-safe and independent
of the recipe's bounded caller depth, unless recipe depth is zero. Graph, snippet,
and token limits still apply and are reported as gaps or omissions. Unregistered
languages remain available as text; this matrix does not promise semantic support
for every language or extension.
