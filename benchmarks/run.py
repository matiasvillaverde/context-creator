#!/usr/bin/env python3
"""Reproducible process-cold ingestion and fixed navigation-task measurements.

No model is called. Each sample receives a fresh copy of the same generated
corpus. OS page caches are not flushed. Optional comparators use argument arrays
from tools.json; installation/download time is excluded from measurements.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import statistics
import subprocess
import tempfile
import time


def corpus(root, count, body_lines):
    """Versioned, deterministic declaration shapes in 29 language families."""
    templates = [
        ("rs", "pub fn {name}(value: i32) -> i32 {{\n{body}    value\n}}\n", "    let _step_{j} = value + {j};\n"),
        ("py", "def {name}(value: int) -> int:\n{body}    return value\n", "    step_{j} = value + {j}\n"),
        ("js", "export function {name}(value) {{\n{body}    return value;\n}}\n", "    const step_{j} = value + {j};\n"),
        ("tsx", "export function {name}(value: number) {{\n{body}    return <span>{{value}}</span>;\n}}\n", "    const step_{j} = value + {j};\n"),
        ("go", "package sample\nfunc {name}(value int) int {{\n{body}    return value\n}}\n", "    _ = value + {j}\n"),
        ("swift", "func {name}(_ value: Int) -> Int {{\n{body}    return value\n}}\n", "    let step_{j} = value + {j}\n"),
        ("java", "class Sample{i} {{ public int {name}(int value) {{\n{body}    return value;\n}} }}\n", "    int step_{j} = value + {j};\n"),
        ("c", "int {name}(int value) {{\n{body}    return value;\n}}\n", "    int step_{j} = value + {j};\n"),
        ("cpp", "int {name}(int value) {{\n{body}    return value;\n}}\n", "    int step_{j} = value + {j};\n"),
        ("cs", "class Sample{i} {{ public int {name}(int value) {{\n{body}    return value;\n}} }}\n", "    int step_{j} = value + {j};\n"),
        ("kt", "fun {name}(value: Int): Int {{\n{body}    return value\n}}\n", "    val step_{j} = value + {j}\n"),
        ("rb", "def {name}(value)\n{body}    value\nend\n", "    step_{j} = value + {j}\n"),
        ("php", "<?php\nfunction {name}(int $value): int {{\n{body}    return $value;\n}}\n", "    $step_{j} = $value + {j};\n"),
        ("zig", "pub fn {name}(value: i32) i32 {{\n{body}    return value;\n}}\n", "    _ = value + {j};\n"),
        ("sh", "{name}() {{\n    local value=${{1:-0}}\n{body}    printf '%s\\n' \"$value\"\n}}\n", "    local step_{j}=$((value + {j}))\n"),
        ("dart", "int {name}(int value) {{\n{body}    return value;\n}}\n", "    final step_{j} = value + {j};\n"),
        ("lua", "function {name}(value)\n{body}    return value\nend\n", "    local step_{j} = value + {j}\n"),
        ("scala", "object Sample{i} {{\ndef {name}(value: Int): Int = {{\n{body}    value\n}}\n}}\n", "    val step_{j} = value + {j}\n"),
        ("ex", "defmodule Sample{i} do\n  def {name}(value) do\n{body}    value\n  end\nend\n", "    step_{j} = value + {j}\n"),
        ("R", "{name} <- function(value) {{\n{body}    value\n}}\n", "    step_{j} <- value + {j}\n"),
        ("jl", "function {name}(value)\n{body}    return value\nend\n", "    step_{j} = value + {j}\n"),
        ("hs", "{name} value =\n  let\n{body}  in value\n", "    step_{j} = value + {j}\n"),
        ("clj", "(defn {name} [value]\n{body}    value)\n", "    (+ value {j})\n"),
        ("ps1", "function {name} {{\n    param($value)\n{body}    return $value\n}}\n", "    $step_{j} = $value + {j}\n"),
        ("pl", "sub {name} {{\n    my ($value) = @_;\n{body}    return $value;\n}}\n", "    my $step_{j} = $value + {j};\n"),
        ("sql", "CREATE TABLE {name} (\n    id INTEGER{body}\n);\n", ",\n    step_{j} INTEGER"),
        ("tf", "resource \"sample\" \"{name}\" {{\n{body}}}\n", "    step_{j} = {j}\n"),
        ("html", "<section id=\"{name}\">\n{body}</section>\n", "    <span>step {j}</span>\n"),
        ("css", ".{name} {{\n{body}}}\n", "    --step-{j}: {j};\n"),
    ]
    tasks = []
    for i in range(count):
        extension, template, statement = templates[i % len(templates)]
        name = f"lookup_customer_{i:04d}"
        relative = f"src/{extension}/sample_{i:04d}.{extension}"
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        body = "".join(statement.format(j=j) for j in range(body_lines))
        path.write_bytes(template.format(name=name, body=body, i=i).encode("utf-8"))
        tasks.append({"symbol": name, "path": relative})
    digest = hashlib.sha256()
    size = 0
    for path in sorted(root.rglob("*")):
        if path.is_file():
            data = path.read_bytes()
            size += len(data)
            digest.update(path.relative_to(root).as_posix().encode() + b"\0" + data + b"\0")
    return {"generator_version": 2, "files": count, "source_bytes": size,
            "body_lines": body_lines, "sha256": digest.hexdigest()}, tasks


def measure(command, root, stdout, environment, timeout=180):
    start = time.perf_counter()
    usage = None
    stderr = stdout.with_suffix(".stderr")
    with stdout.open("wb") as output, stderr.open("wb") as errors:
        process = subprocess.Popen(command, cwd=root, stdout=output, stderr=errors, env=environment)
        try:
            if hasattr(os, "wait4"):
                # wait4 supplies per-process CPU/RSS without an external `time`
                # executable or polling delay. The timer bounds a stuck tool.
                def expired(_signum, _frame):
                    raise subprocess.TimeoutExpired(command, timeout)
                previous = signal.signal(signal.SIGALRM, expired)
                signal.setitimer(signal.ITIMER_REAL, timeout)
                try:
                    _, status, usage = os.wait4(process.pid, 0)
                    process.returncode = os.waitstatus_to_exitcode(status)
                finally:
                    signal.setitimer(signal.ITIMER_REAL, 0)
                    signal.signal(signal.SIGALRM, previous)
            else:
                process.wait(timeout=timeout)
        except BaseException:
            process.kill()
            process.wait()
            raise
    wall = time.perf_counter() - start
    if process.returncode:
        raise RuntimeError(f"exit {process.returncode}: {stderr.read_text(errors='replace')[-2000:]}")
    cpu = usage.ru_utime + usage.ru_stime if usage else None
    rss = usage.ru_maxrss if usage else None
    if rss is not None and platform.system() == "Darwin":
        rss /= 1024  # macOS reports bytes; Linux reports KiB.
    return {"wall_seconds": wall, "cpu_seconds": cpu, "peak_rss_kib": rss}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/context-creator")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--files", type=int, default=290)
    parser.add_argument("--body-lines", type=int, default=40)
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--map-tokens", type=int, default=4096)
    parser.add_argument("--comparators", action="store_true")
    parser.add_argument("--tools", type=Path, default=Path(__file__).with_name("tools.json"))
    args = parser.parse_args()
    if min(args.files, args.body_lines, args.repeats, args.map_tokens) < 1:
        parser.error("counts must be positive")
    binary = str(Path(args.binary).resolve())
    version = subprocess.check_output([binary, "--version"], text=True).strip()
    try:
        import tiktoken
        encoding = tiktoken.get_encoding("o200k_base")
    except ImportError:
        encoding = None
        if args.comparators:
            parser.error("comparators require the pinned tiktoken package in requirements.txt")
    environment = dict(os.environ)
    # All measurements operate on local generated data; no model credentials,
    # analytics, or model-metadata downloads are needed by the adapters.
    for key in list(environment):
        if key.endswith(("_API_KEY", "_TOKEN")):
            environment.pop(key)
    environment.update({"LITELLM_LOCAL_MODEL_COST_MAP": "True", "AIDER_ANALYTICS": "false", "LC_ALL": "C"})
    repo = Path(__file__).resolve().parent.parent
    tools = [
        {"name": "context-creator/full", "version": version, "artifact": "file", "own": True,
         "command": ["{binary}", "{root}", "--quiet", "--output", "{output}", "--report", "{report}"]},
        {"name": "context-creator/map", "version": version, "artifact": "file", "own": True,
         "command": ["{binary}", "{root}", "--quiet", "--style", "map", "--max-tokens", "{tokens}",
                     "--output", "{output}", "--report", "{report}"]},
    ]
    if args.comparators:
        tools.extend(json.loads(args.tools.read_text()))
    rows = []
    with tempfile.TemporaryDirectory(prefix="context-benchmark-") as temporary:
        temporary = Path(temporary)
        original = temporary / "corpus"
        original.mkdir()
        manifest, tasks = corpus(original, args.files, args.body_lines)
        for tool in tools:
            observed_version = version
            if tool.get("version_command"):
                observed_version = subprocess.check_output(tool["version_command"],
                    env=environment, text=True, timeout=60).strip()
                if not re.search(r"(?<![\d.])" + re.escape(tool["version"]) + r"(?![\d.])", observed_version):
                    raise SystemExit(f"{tool['name']}: expected {tool['version']}, found {observed_version}")
            samples = []
            error = None
            for index in range(args.repeats):
                run = temporary / "sample"
                if run.exists():
                    shutil.rmtree(run)
                root = run / "repo"
                shutil.copytree(original, root)
                values = {"root": str(root), "output": str(run / "context.txt"),
                          "report": str(run / "report.json"), "tokens": str(args.map_tokens),
                          "repo": str(repo), "binary": binary}
                command = [argument.format(**values) for argument in tool["command"]]
                try:
                    sample = measure(command, root, run / "stdout.txt", environment)
                    artifact = run / ("stdout.txt" if tool["artifact"] == "stdout" else "context.txt")
                    text = artifact.read_bytes().decode("utf-8")
                    sample["output_bytes"] = len(text.encode())
                    sample["navigation_tasks_passed"] = sum(
                        task["symbol"] in text and task["path"] in text for task in tasks)
                    sample["navigation_tasks_total"] = len(tasks)
                    if tool.get("own"):
                        report = json.loads((run / "report.json").read_text())
                        sample["tokens"] = report["context_tokens"]
                        sample["files_selected"] = report["files_selected"]
                        sample["timings_ms"] = report["timings_ms"]
                        if encoding:
                            assert len(encoding.encode_ordinary(text)) == sample["tokens"]
                    else:
                        sample["tokens"] = len(encoding.encode_ordinary(text))
                    samples.append(sample)
                except (OSError, RuntimeError, subprocess.TimeoutExpired, AssertionError) as failure:
                    error = str(failure) or type(failure).__name__
                    break
            row = {"tool": tool["name"], "version": tool["version"],
                   "command": tool["command"], "observed_version": observed_version,
                   "samples": samples, "error": error}
            if samples:
                row["median_wall_seconds"] = statistics.median(sample["wall_seconds"] for sample in samples)
            rows.append(row)
            print(f"{tool['name']}: {row.get('median_wall_seconds', 'failed')} s", flush=True)
    revision = subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"],
                              capture_output=True, text=True, check=False).stdout.strip() or None
    processor = platform.processor() or platform.machine()
    if platform.system() == "Linux":
        cpuinfo = Path("/proc/cpuinfo").read_text()
        processor = next((line.partition(":")[2].strip() for line in cpuinfo.splitlines()
                          if line.startswith("model name")), processor)
    result = {"schema_version": 1, "corpus": manifest, "tokenizer": "o200k-base",
              "source_commit": revision,
              "binary_sha256": hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
              "platform": platform.platform(), "processor": processor,
              "logical_cpus": os.cpu_count(), "process_cache": "cold; fresh corpus copy per sample",
              "filesystem_cache": "uncontrolled; OS caches are not flushed",
              "quality_metric": "symbol + source-path recall on fixed navigation tasks; no LLM answers",
              "tools": rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    if any(row["error"] is not None for row in rows):
        raise SystemExit("One or more measurements failed; see the JSON report")


if __name__ == "__main__":
    main()
