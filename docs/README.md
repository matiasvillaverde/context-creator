# context-creator documentation

context-creator provides a Rust crate, CLI, and MCP server for building tools
that give agents selected repository source within a token budget. Start with
[the README quickstart](../README.md#quickstart) to try the included example.

## Choose a workflow

| Goal | Guide |
| --- | --- |
| Install the current source or a released package | [Installation](installation.md) |
| Select context for a task or seed file | [Agent tools](agent-tools.md#task-context) |
| Embed the Rust API | [Crate integration](agent-tools.md#crate-integration) |
| Connect an MCP client | [MCP server](mcp-server.md) |
| Define reusable tools | [Recipes](agent-tools.md#recipes) |
| Update an agent's context after edits | [Incremental sessions](agent-tools.md#incremental-sessions) |
| Collect a change and its likely impact | [Change impact](agent-tools.md#change-impact) |
| Follow literal HTTP/schema relationships | [Contract hints](agent-tools.md#connections-across-services-and-languages) |
| Configure filters, budgets, and secret handling | [Configuration](configuration.md) |

Run `context-creator --help` and `context-creator context --help` for the flags
available in your installed binary. The README's examples target `main`;
published releases can have fewer features.

## Understand the results

- [Language support](languages.md) describes extraction and local resolution by family.
- [Analysis limits](agent-tools.md#explanations-limits-and-safety) explain confidence,
  gaps, omissions, filtering, and request limits.
- [The open-source audit](oss-language-audit.md) records 87 random files from
  29 projects, manual checks, successful cases, and confirmed defects.
- [Reproducible measurements](../benchmarks/README.md) include pinned retrieval
  experiments and their methodology.

Parser availability does not establish complete feature coverage. Task matching
and dependency links use lexical/static evidence; callers and contract matches
can be inferred. Inspect each response's gaps and omissions.

## Development and research

- [Contributing](../CONTRIBUTING.md) — Setup, conventions, and checks
- [Source map and agent guidance](../AGENTS.md) — Current modules and architecture rules
- [Modernization research](modernization.md) — Product direction and tool comparisons
- [README comparison](readme-review.md) — Eight upstream READMEs and the choices made here
