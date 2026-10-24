# Reproducible context measurements

Build the release binary before measuring:

```bash
cargo build --release --locked
python3 benchmarks/run.py --output /tmp/context-results.json
```

The versioned generator creates a deterministic corpus across thirteen language
families. Reports include its SHA-256, file/source/output sizes, local tokens,
CPU time and peak RSS on Linux, per-stage context-creator timings, and recall of
fixed symbol-plus-path navigation tasks. Each process gets a fresh corpus copy
and application caches. OS filesystem caches are uncontrolled. No LLM is called,
so navigation recall is not a measure of generated-answer quality.

For the pinned cross-tool comparison, install packages in an isolated Python
3.12 environment and place Code2prompt 4.2.0 on its PATH. Node/npm supplies
Repomix 1.18.1 through `npx`:

```bash
uv venv --python 3.12 /tmp/context-bench-env
uv pip install --python /tmp/context-bench-env/bin/python -r benchmarks/requirements.txt
PATH="/tmp/context-bench-env/bin:$PATH" /tmp/context-bench-env/bin/python benchmarks/run.py \
  --comparators --files 130 --body-lines 40 --repeats 5 \
  --output /tmp/context-comparison.json
```

[tools.json](tools.json) records exact argument arrays and version checks.
`--tools PATH` accepts alternate executable locations. Dependency installation,
tool version checks, corpus creation, and normalized token recounting happen
outside the measured process. All output is recounted with `o200k_base`.
Code2prompt uses its supported `cl100k` encoding internally; its count is not
used as the normalized output count. Native tokenization/reporting work and
output envelopes differ between tools, so elapsed times describe these commands,
rather than an isolated implementation speed ranking. Context-creator includes
JSON reporting. Repomix's optional security scan is disabled on this synthetic
corpus, matching context-creator's default scanning policy.

The Aider adapter runs the pinned release's actual `RepoMap` implementation,
including ranking, tag extraction, and rendering. It uses a local tokenizer
instead of creating a model client and bypasses interactive CLI startup. No
model credentials are passed to comparison processes. Comparisons fail if an
installed version differs or a tool fails; failures remain visible in JSON.

Measure in-process declaration cache reuse separately:

```bash
cargo bench --bench benchmarks repository_map
```

`cold_symbols` starts with a fresh file/declaration cache, while `warm_symbols`
reuses loaded source and AST-derived declarations. Both use an initialized BPE
vocabulary. Existing benchmarks separately cover walking, token-cache hits,
first-time encoding, prioritization, full rendering, and type resolution.
