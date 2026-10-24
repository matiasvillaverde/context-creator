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
import statistics
import subprocess
import tempfile
import time


def corpus(root, count, body_lines):
    """Versioned, deterministic, valid declaration shapes in 13 languages."""
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
    ]
    tasks = []
    for i in range(count):
        extension, template, statement = templates[i % len(templates)]
        name = f"lookup_customer_{i:04d}"
        relative = f"src/{extension}/sample_{i:04d}.{extension}"
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        body = "".join(statement.format(j=j) for j in range(body_lines))
        path.write_text(template.format(name=name, body=body, i=i), encoding="utf-8")
        tasks.append({"symbol": name, "path": relative})
    digest = hashlib.sha256()
    size = 0
    for path in sorted(root.rglob("*")):
        if path.is_file():
            data = path.read_bytes()
            size += len(data)
            digest.update(path.relative_to(root).as_posix().encode() + b"\0" + data + b"\0")
    return {"generator_version": 1, "files": count, "source_bytes": size,
            "body_lines": body_lines, "sha256": digest.hexdigest()}, tasks


def measure(command, root, stdout, metrics, environment):
    has_time = platform.system() == "Linux" and Path("/usr/bin/time").exists()
    if has_time:
        command = ["/usr/bin/time", "-f", "%U %S %M", "-o", str(metrics), *command]
    start = time.perf_counter()
    with stdout.open("wb") as output:
        result = subprocess.run(command, cwd=root, stdout=output, stderr=subprocess.PIPE,
                                env=environment, timeout=180, check=False)
    wall = time.perf_counter() - start
    if result.returncode:
        raise RuntimeError(f"exit {result.returncode}: {result.stderr.decode(errors='replace')[-2000:]}")
    cpu, rss = None, None
    if has_time:
        user, system, peak = metrics.read_text().strip().split()
        cpu, rss = float(user) + float(system), int(peak)
    return {"wall_seconds": wall, "cpu_seconds": cpu, "peak_rss_kib": rss}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/context-creator")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--files", type=int, default=130)
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
    environment.update({"LITELLM_LOCAL_MODEL_COST_MAP": "True", "AIDER_ANALYTICS": "false"})
    repo = Path(__file__).resolve().parent.parent
    tools = [
        {"name": "context-creator/full", "version": version, "artifact": "file", "own": True,
         "command": [binary, "{root}", "--quiet", "--output", "{output}", "--report", "{report}"]},
        {"name": "context-creator/map", "version": version, "artifact": "file", "own": True,
         "command": [binary, "{root}", "--quiet", "--style", "map", "--max-tokens", "{tokens}",
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
                          "repo": str(repo)}
                command = [argument.format(**values) for argument in tool["command"]]
                try:
                    sample = measure(command, root, run / "stdout.txt", run / "metrics", environment)
                    artifact = run / ("stdout.txt" if tool["artifact"] == "stdout" else "context.txt")
                    text = artifact.read_text(encoding="utf-8")
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
                    error = str(failure)
                    break
            row = {"tool": tool["name"], "version": tool["version"],
                   "command": tool["command"], "observed_version": observed_version,
                   "samples": samples, "error": error}
            if samples:
                row["median_wall_seconds"] = statistics.median(sample["wall_seconds"] for sample in samples)
            rows.append(row)
            print(f"{tool['name']}: {row.get('median_wall_seconds', 'failed')} s", flush=True)
    result = {"schema_version": 1, "corpus": manifest, "tokenizer": "o200k-base",
              "platform": platform.platform(), "processor": platform.processor(),
              "logical_cpus": os.cpu_count(), "process_cache": "cold; fresh corpus copy per sample",
              "filesystem_cache": "uncontrolled; OS caches are not flushed",
              "quality_metric": "symbol + source-path recall on fixed navigation tasks; no LLM answers",
              "tools": rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    if any(row["error"] for row in rows):
        raise SystemExit("One or more measurements failed; see the JSON report")


if __name__ == "__main__":
    main()
