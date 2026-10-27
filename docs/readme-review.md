# README comparison — 2026-10-27

The goal was to make context-creator easier to understand and try, particularly
for developers building tools for agents. We reviewed eight complete current
English/root READMEs: the three projects named in the request, Grafana CLI's
successor, two context tools, and two widely used CLIs.

Exa supplied discovery results; pinned GitHub content supplied the full READMEs
when indexed/fetched copies were truncated or older. The two research workstreams
requested 35 Exa results in total. Those are search-result slots, not 35 distinct
READMEs read in full. No upstream installer or application was executed.

These projects are useful references, but their popularity does not establish
that a particular README layout caused adoption. The recommendations below are
editorial judgments about onboarding and navigation.

## Sources and observations

Links point to the snapshots reviewed, rather than moving branches. Line counts
are approximate raw Markdown counts.

| README | Lines | Useful pattern | What we did not adopt |
| --- | ---: | --- | --- |
| [OpenClaw](https://github.com/openclaw/openclaw/blob/183e4943702c46ee233a5a0248507daa0f1ea5a4/README.md) | 331 | Names the audience and setting; installation and onboarding lead to an explicit check that a message works. Task-oriented links help readers choose documentation. | Its long governance/community material would delay this smaller tool's first use. |
| [OpenCode](https://github.com/anomalyco/opencode/blob/dae4077a96243257ea0a2086068504b791629f10/README.md) | 114 | A short category statement and terminal screenshot make the product recognizable; references stay brief. | This snapshot delegates launch and provider setup to external docs. We keep an executable first run in the README. |
| [Grafana CLI: grafanactl](https://github.com/grafana/grafanactl/blob/9bbefbd1e061a6943587e8c69494c1da5ada2337/README.md) | 31 | Concrete automation/environment use cases and explicit maturity/support boundaries. | The deprecation notice points to gcx. There is no inline installation or first-use example to model our onboarding on. |
| [Grafana CLI: gcx](https://github.com/grafana/gcx/blob/69e53179d04898cd5d7534f5da2f8c416894f924/README.md) | 508 | Separates CLI, MCP, and assistant routes; commands, result tables, and graphs demonstrate outcomes. | Compatibility matrices and the long incident narrative would crowd out our quickstart. |
| [Repomix](https://github.com/yamadashy/repomix/blob/a835271f41701245ee24b3c8ba162c5abb3d6f12/README.md) | 2,027 | Explains the repository-to-context artifact clearly, names its output, and tells users what to do with it. | Promotional material and a full reference manual make the entry point long. |
| [Gitingest](https://github.com/coderamp-labs/gitingest/blob/4e259a02fe72115bee538271622f1234a81c8e1a/README.md) | 356 | Distinguishes CLI output files from programmatic API return values. | Browser-extension and hosting instructions are not useful routes for our product. |
| [GitHub CLI](https://github.com/cli/cli/blob/63837fb778d6a39f02be0ce76ff89f2bc8d74bdc/README.md) | 123 | Familiar jobs, visible command results, and selective links keep navigation compact. | Installation is mostly delegated to platform pages; our current source needs an explicit runnable path. |
| [ripgrep](https://github.com/BurntSushi/ripgrep/blob/6892bc2b7efe5909cbbeec8f757ed2c4c76b1d30/README.md) | 542 | States defaults and reasons a tool may not fit; performance evidence has commands, a corpus, and qualifications. | Benchmark commands appear before beginner installation. Our first command should establish a useful result. |

The grafanactl snapshot announces deprecation and gives a June 1, 2026 archive
date. GitHub still reported `archived: false` at review time; we describe the
notice without claiming the repository has actually been archived. Current
OpenCode and GitHub CLI READMEs also differed from search-index excerpts, which
is why the pinned files are the final sources.

## Changes to our README

1. **Lead with the outcome.** The first screen names relevant code, a token
   budget, agent-tool builders, and the crate/CLI/MCP interfaces. Local processing
   and model handoff are separate capabilities.
2. **Make the first run reproducible.** Installation precedes feature details.
   A bundled Zig task gives users valid input without a Zig compiler, model key,
   or a repository-specific filename guess. The selected-symbol preview comes
   from the running CLI, with JSON stdout and reported external imports explained.
3. **Show the important workflows.** Task context, change review, repository
   maps, artifacts, and revision-checked sessions explain what an agent can do.
   Low-level dependency and protocol versions move out of the feature pitch.
4. **Give each integration a route.** CLI users start in the quickstart; MCP
   clients get a native server configuration and request parameters; Rust users
   get the shared `AgentService` API. Client configuration formats are qualified.
5. **Use the guides for depth.** Complete tool catalogs, recipes, session cursor
   rules, provider accounting, and configuration belong in their existing guides.
   The documentation index now points to current workflows and evidence.
6. **Keep limits visible.** Language registration is distinguished from feature
   coverage. Known audit failures, static/inferred relationships, token-envelope
   boundaries, and large-project costs remain visible beside the capabilities.
7. **Align adjacent setup instructions.** The installation guide distinguishes
   current `main` APIs from published packages; contributor Rust requirements
   match `Cargo.toml`. No product behavior or install mechanism was added.

The rewritten root README is 233 lines, down from 346. Shortness is a secondary
result; the primary goal is a clear route from a reader's task to a verified
command and an understandable result. No claim about conversion, retrieval
accuracy, or agent task-completion quality follows from these documentation edits.

## Validation

Installed the native CLI into an isolated temporary directory. All six CLI
commands in the README passed using that binary, including the bundled task,
seeded context, map, change-impact request against a temporary Git fixture,
artifact/report generation, and recipe listing. The symbol preview was checked
against the returned paths, locations, and selection reasons.

The documented server command and request passed an MCP stdio initialization
and `build_context` call. The Rust snippet compiled and ran against the checked-out
API and its locked dependency versions. All 44 local links and heading anchors
in the updated landing pages resolved. Documentation changes do not establish that the product defects in the
open-source audit have been fixed.
