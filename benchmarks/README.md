# Reproducible context measurements

Build the release binary before measuring:

```bash
cargo build --release --locked
python3 benchmarks/run.py --output /tmp/context-results.json
```

The versioned generator creates a deterministic corpus across thirteen language
families. Reports include its SHA-256, file/source/output sizes, local tokens,
CPU time and peak RSS on Unix, per-stage context-creator timings, and recall of
fixed symbol-plus-path navigation tasks. Each process gets a fresh corpus copy
and application caches. OS filesystem caches are uncontrolled. No LLM is called,
so navigation recall is not a measure of generated-answer quality.

For the pinned cross-tool comparison, install packages in an isolated Python
3.12 environment and place Code2prompt 4.2.0 on its PATH. Install Repomix in a
temporary prefix so measurement invokes its Node entry point directly, excluding
`npx` package-runner overhead:

```bash
uv venv --python 3.12 /tmp/context-bench-env
uv pip install --python /tmp/context-bench-env/bin/python -r benchmarks/requirements.txt
npm install --prefix /tmp/context-bench-node repomix@1.18.1
PATH="/tmp/context-bench-env/bin:/tmp/context-bench-node/node_modules/.bin:$PATH" /tmp/context-bench-env/bin/python benchmarks/run.py \
  --comparators --files 130 --body-lines 40 --repeats 5 \
  --output /tmp/context-comparison.json
```

[tools.json](tools.json) records exact argument arrays and version checks.
`--tools PATH` accepts alternate executable locations. Dependency installation,
tool version checks, corpus creation, and normalized token recounting happen
outside the measured process. Unix measurements use `wait4` for per-process resource usage; tools time out after
180 seconds. All output is recounted with `o200k_base`.
Code2prompt uses its supported `cl100k` encoding internally; its count is not
used as the normalized output count. Native tokenization/reporting work and
output envelopes differ between tools, so elapsed times describe these commands,
rather than an isolated implementation speed ranking. Context-creator includes
JSON reporting. Repomix's optional security scan is disabled on this synthetic
corpus, matching context-creator's default scanning policy.

The Aider adapter runs the pinned release's actual `RepoMap` implementation,
including ranking, tag extraction, and rendering. It uses a local tokenizer
instead of creating a model client and bypasses interactive CLI startup. No
model credentials are passed to comparison processes. Version mismatches stop
before measurement. Tool runtime failures remain visible in JSON and produce a
nonzero exit code. Reports record the source commit and binary SHA-256.

Measure in-process declaration cache reuse separately:

```bash
cargo bench --bench benchmarks repository_map
```

`cold_symbols` starts with a fresh file/declaration cache, while `warm_symbols`
reuses loaded source and AST-derived declarations. Both use an initialized BPE
vocabulary. Existing benchmarks separately cover walking, token-cache hits,
first-time encoding, prioritization, full rendering, and type resolution.

The [2026-10-24 dataset](results/2026-10-24.json) and its [interpretation](../docs/modernization.md#release-measurements-2026-10-24) are checked in. Raw build outputs and generated corpora are not tracked.

For task selection and incremental sessions, run:

```bash
cargo run --release --locked --example agent_workflow
```

The experiment uses one task, five expected source/test/configuration paths, and
120 distractor files. It checks full-response token budgets, unchanged-session
content reuse, and a single-file edit. JSON reports cold/warm/edited latency,
response tokens, delivered content bytes, and analysis reuse. Tokenizer
initialization is excluded; source scanning and final JSON accounting are included.
These are local retrieval checks, not live model task-completion scores.

The [2026-10-26 agent-context dataset](results/2026-10-26-agent-context.json)
records five fresh release processes at source `cf423a0`, with the executable
SHA-256, host, every sample, CPU time, and peak RSS. The corpus has 127 files and
443,235 source bytes; five expected paths were retained in every stage/sample.
Per-request median results on this host were:

| Request | Wall ms | Complete response tokens | Source bytes delivered |
| --- | ---: | ---: | ---: |
| Cold task context | 434.0 | 2,195 | 4,141 |
| Unchanged session | 22.8 | 1,320 | 0 |
| Single-file edit | 173.8 | 2,071 | 3,697 |

The edit changes four source bytes; the delta replaces the complete changed
function. Unchanged requests still send snippet identities, current locations,
reasons, and relationship metadata. OS caches were not flushed. Edited requests
rebuild static relationships, so they cost more than unchanged requests. These
numbers describe this fixture and host, not arbitrary repository latency.
