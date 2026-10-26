#!/usr/bin/env python3
"""Pin, randomly sample, and inspect public source without running upstream code.

The cache and raw reports belong outside the checkout. Network access is needed
only for prepare; run uses the pinned extracted files and local Rust binaries.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import random
import secrets
import shutil
import subprocess
import tarfile
import tempfile
import time
import urllib.request


PROJECTS = [
    ("Rust", "tokio-rs/tokio", ["rs"]),
    ("Python", "pallets/flask", ["py", "pyi"]),
    ("JavaScript", "expressjs/express", ["js", "jsx", "mjs", "cjs"]),
    ("TypeScript", "colinhacks/zod", ["ts", "tsx", "mts", "cts"]),
    ("Go", "gin-gonic/gin", ["go"]),
    ("Swift", "Alamofire/Alamofire", ["swift"]),
    ("Java", "junit-team/junit5", ["java"]),
    ("C", "redis/redis", ["c"]),
    ("C++", "fmtlib/fmt", ["cc", "cpp", "cxx", "h", "hpp", "hh", "hxx"]),
    ("C#", "serilog/serilog", ["cs"]),
    ("Kotlin", "square/okhttp", ["kt", "kts"]),
    ("Ruby", "jekyll/jekyll", ["rb"]),
    ("PHP", "laravel/framework", ["php"]),
    ("Zig", "zigtools/zls", ["zig"]),
    ("Bash", "ohmybash/oh-my-bash", ["sh", "bash"]),
    ("Dart", "cfug/dio", ["dart"]),
    ("Lua", "neovim/neovim", ["lua"]),
    ("Scala", "typelevel/cats", ["scala", "sc"]),
    ("Elixir", "phoenixframework/phoenix", ["ex", "exs"]),
    ("R", "tidyverse/dplyr", ["r"]),
    ("Julia", "FluxML/Flux.jl", ["jl"]),
    ("Haskell", "jgm/pandoc", ["hs"]),
    ("Clojure", "ring-clojure/ring", ["clj", "cljs", "cljc"]),
    ("PowerShell", "pester/Pester", ["ps1", "psm1", "psd1"]),
    ("Perl", "mojolicious/mojo", ["pl", "pm"]),
    ("SQL", "pgvector/pgvector", ["sql"]),
    ("HCL", "terraform-aws-modules/terraform-aws-vpc", ["tf", "tfvars", "hcl"]),
    ("HTML", "mdn/dom-examples", ["html", "htm"]),
    ("CSS", "animate-css/animate.css", ["css"]),
]
EXCLUDED_DIRS = {"node_modules", "target", "vendor", "dist", "build", "venv"}
MAX_SOURCE_BYTES = 65_536
MAX_SOURCE_LINES = 600


def digest(data):
    return hashlib.sha256(data).hexdigest()


def save(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")


def confined_path(root, relative):
    parts = PurePosixPath(relative)
    if parts.is_absolute() or ".." in parts.parts or "\\" in relative:
        raise ValueError("Audit paths must remain relative to their cache root")
    path = root.resolve().joinpath(*parts.parts)
    if not path.resolve().is_relative_to(root.resolve()):
        raise ValueError("Audit path escapes its cache root")
    return path


def record_run(args, mode):
    repository = Path(__file__).resolve().parent.parent
    commit = subprocess.run(["git", "-C", str(repository), "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
    source_matches = subprocess.run(["git", "-C", str(repository), "diff", "--no-ext-diff", "--no-textconv", "--quiet", "HEAD", "--", "src", "Cargo.toml", "Cargo.lock"], check=False).returncode == 0
    save(args.output / f"run.{mode}.json", {
        "source_commit": commit, "product_sources_match_commit": source_matches,
        "tool_binary_sha256": digest(args.binary.read_bytes()) if args.binary.is_file() else None,
        "probe_binary_sha256": digest(args.probe.read_bytes()) if args.probe.is_file() else None,
        "platform": platform.platform(), "parallel_projects": 3,
        "global_git_config": "user" if args.user_git_config else "child-only neutral configuration",
        "live_models_called": False, "upstream_application_code_executed": False,
    })


def github(endpoint):
    result = subprocess.run(["gh", "api", endpoint], capture_output=True, text=True, timeout=60)
    if result.returncode:
        raise RuntimeError(result.stderr.strip())
    return json.loads(result.stdout)


def download(project, cache):
    destination = confined_path(cache, project["cache_key"])
    receipt = destination / ".audit-receipt.json"
    if receipt.is_file():
        assert json.loads(receipt.read_text())["commit"] == project["commit"]
        return destination
    url = f'https://codeload.github.com/{project["repository"]}/tar.gz/{project["commit"]}'
    with tempfile.TemporaryDirectory(prefix="oss-download-", dir=cache) as temporary:
        temporary = Path(temporary)
        archive = temporary / "source.tar.gz"
        request = urllib.request.Request(url, headers={"User-Agent": "context-creator-source-audit"})
        with urllib.request.urlopen(request, timeout=90) as response, archive.open("wb") as output:
            total = 0
            while chunk := response.read(1_048_576):
                total += len(chunk)
                if total > 128 * 1_048_576:
                    raise ValueError("Archive exceeds 128 MiB download limit")
                output.write(chunk)
        extracted = temporary / "source"
        extracted.mkdir()
        total = 0
        skipped = 0
        with tarfile.open(archive, "r:gz") as source:
            for member in source:
                path = PurePosixPath(member.name)
                if path.is_absolute() or ".." in path.parts or "\\" in member.name:
                    raise ValueError(f"Unsafe archive member: {member.name!r}")
                if not member.isfile() or len(path.parts) < 2:
                    continue
                if member.size > 10 * 1_048_576:
                    skipped += 1
                    continue
                total += member.size
                if total > 512 * 1_048_576:
                    raise ValueError("Extracted source exceeds 512 MiB limit")
                target = extracted.joinpath(*path.parts[1:])
                target.parent.mkdir(parents=True, exist_ok=True)
                stream = source.extractfile(member)
                if stream is None:
                    raise ValueError("Archive member has no file content")
                with stream, target.open("wb") as output:
                    shutil.copyfileobj(stream, output)
        save(extracted / ".audit-receipt.json", {
            "commit": project["commit"], "archive_sha256": digest(archive.read_bytes()),
            "source_bytes": total, "oversized_members_skipped": skipped,
            "symlinks_and_special_files": "not extracted",
        })
        if destination.exists():
            raise ValueError(f"Incomplete cache already exists: {destination}")
        extracted.rename(destination)
    return destination


def candidates(root, extensions):
    found = []
    for directory, dirs, files in os.walk(root, followlinks=False):
        dirs[:] = sorted(name for name in dirs if not name.startswith(".") and name not in EXCLUDED_DIRS)
        for name in sorted(files):
            path = Path(directory) / name
            if name.startswith(".") or path.suffix.lstrip(".").lower() not in extensions or path.is_symlink():
                continue
            data = path.read_bytes()
            if not 0 < len(data) <= MAX_SOURCE_BYTES or len(data.splitlines()) > MAX_SOURCE_LINES or b"\0" in data:
                continue
            try:
                data.decode("utf-8")
            except UnicodeDecodeError:
                continue
            found.append(path.relative_to(root).as_posix())
    return sorted(found)


def prepare(args):
    args.cache.mkdir(parents=True, exist_ok=True)
    if args.manifest.exists():
        manifest = json.loads(args.manifest.read_text())
    else:
        manifest = {
            "schema_version": 1, "seed": secrets.token_hex(16), "samples_per_language": 3,
            "selection": "random.Random(seed + language).sample(sorted(eligible_paths), 3); no redraws",
            "eligibility": {"max_bytes": MAX_SOURCE_BYTES, "max_lines": MAX_SOURCE_LINES,
                            "encoding": "UTF-8", "hidden_and_symlink_files": "excluded",
                            "excluded_directories": sorted(EXCLUDED_DIRS), "test_files": "eligible"},
            "projects": [],
        }
        for language, repository, extensions in PROJECTS:
            metadata = github(f"repos/{repository}")
            repository = metadata["full_name"]
            commit = github(f"repos/{repository}/commits/HEAD")["sha"]
            project = {
                "language": language, "repository": repository, "commit": commit,
                "url": metadata["html_url"], "stars_at_selection": metadata["stargazers_count"],
                "license": (metadata.get("license") or {}).get("spdx_id"), "extensions": extensions,
                "cache_key": repository.replace("/", "--") + "--" + commit[:12],
            }
            manifest["projects"].append(project)
            print(f"Pinned {language}: {repository}@{commit[:12]}", flush=True)
        save(args.manifest, manifest)
    with ThreadPoolExecutor(max_workers=4) as pool:
        work = {pool.submit(download, p, args.cache): p for p in manifest["projects"]}
        for future in as_completed(work):
            project = work[future]
            root = future.result()
            paths = candidates(root, project["extensions"])
            fingerprint = digest("\n".join(paths).encode())
            if "candidate_pool_sha256" in project:
                assert project["candidate_pool_sha256"] == fingerprint, project["language"]
            else:
                project["candidate_pool_sha256"] = fingerprint
                project["candidate_count"] = len(paths)
                chosen = random.Random(manifest["seed"] + project["language"]).sample(paths, min(3, len(paths)))
                project["samples"] = [{"path": path, "sha256": digest((root / path).read_bytes()),
                                       "bytes": (root / path).stat().st_size,
                                       "lines": len((root / path).read_bytes().splitlines())} for path in chosen]
            print(f'Sampled {project["language"]}: {len(project["samples"])} / {len(paths)} eligible', flush=True)
    save(args.manifest, manifest)


def audit_environment(args):
    environment = os.environ.copy()
    if not args.user_git_config:
        settings = args.cache / ".audit-environment"
        settings.mkdir(parents=True, exist_ok=True)
        excludes = settings / "empty-ignore"
        excludes.write_text("")
        config = settings / "gitconfig"
        config.write_text("[core]\n\texcludesFile = " + json.dumps(str(excludes.resolve()), ensure_ascii=False) + "\n")
        environment["GIT_CONFIG_GLOBAL"] = str(config.resolve())
        environment["GIT_CONFIG_NOSYSTEM"] = "1"
    return environment


def run_cli(binary, arguments, root, environment):
    started = time.monotonic()
    try:
        result = subprocess.run([str(binary), *arguments], cwd=root, env=environment, capture_output=True, text=True, timeout=90)
        return {"arguments": arguments, "exit_code": result.returncode, "seconds": time.monotonic() - started,
                "stdout": result.stdout, "stderr": result.stderr}
    except subprocess.TimeoutExpired:
        return {"arguments": arguments, "error": "90 second timeout", "seconds": time.monotonic() - started}


def run_project(args, project):
    root = confined_path(args.cache, project["cache_key"]).resolve()
    for index, sample in enumerate(project["samples"]):
        path = confined_path(root, sample["path"])
        assert digest(path.read_bytes()) == sample["sha256"], path
        key = project["language"].replace("+", "p").replace("#", "sharp").lower() + f"-{index}"
        result = {"language": project["language"], "repository": project["repository"], "sample": sample,
                  "root": str(root), "key": key}
        result["map"] = run_cli(args.binary, [str(path), "--quiet", "--style", "map", "--max-tokens", "4096", "--secret-scan", "off"], root, args.environment)
        if index == 0:
            result["expansion"] = run_cli(args.binary, [str(path), "--quiet", "--style", "paths", "--trace-imports", "--include-callers", "--include-types", "--semantic-depth", "2", "--max-tokens", "8192", "--secret-scan", "off"], root, args.environment)
        save(args.output / f"{key}.cli.json", result)
        print(f'{project["language"]} {sample["path"]}: map={result["map"].get("exit_code", result["map"].get("error"))}', flush=True)


def run(args):
    manifest = json.loads(args.manifest.read_text())
    args.output.mkdir(parents=True, exist_ok=True)
    args.environment = audit_environment(args)
    record_run(args, "cli")
    with ThreadPoolExecutor(max_workers=3) as pool:
        work = [pool.submit(run_project, args, project) for project in manifest["projects"] if args.language is None or project["language"] == args.language]
        for future in as_completed(work):
            future.result()


def probe_project(args, project):
    started = time.monotonic()
    root = confined_path(args.cache, project["cache_key"])
    originals = {confined_path(root, sample["path"]): confined_path(root, sample["path"]).read_bytes() for sample in project["samples"]}
    for sample in project["samples"]:
        assert digest(originals[root / sample["path"]]) == sample["sha256"]
    # A timed-out repeat must not leave an older successful result in the report.
    family = project["language"].replace("+", "p").replace("#", "sharp").lower()
    suffix = "semantic" if args.mode == "full" else args.mode
    for index in range(len(project["samples"])):
        (args.output / f"{family}-{index}.{suffix}.json").unlink(missing_ok=True)
    command = [str(args.probe), str(args.manifest.resolve()), str(args.cache.resolve()), str(args.output.resolve()), project["language"], args.mode]
    limit = {"full": 420, "sessions": 180, "symbols": 45}[args.mode]
    try:
        result = subprocess.run(command, env=args.environment, capture_output=True, text=True, timeout=limit)
        receipt = {"exit_code": result.returncode, "stderr": result.stderr, "seconds": time.monotonic() - started}
    except subprocess.TimeoutExpired as error:
        receipt = {"error": f"{limit} second timeout", "seconds": time.monotonic() - started,
                   "stderr": (error.stderr or b"").decode(errors="replace")}
    finally:
        for path, original in originals.items():
            if path.read_bytes() != original:
                path.write_bytes(original)
    save(args.output / (project["cache_key"] + f".{args.mode}.probe.json"), receipt)
    print(f'{project["language"]} semantic probe: {receipt.get("exit_code", receipt.get("error"))} ({receipt["seconds"]:.1f}s)', flush=True)


def probe(args):
    manifest = json.loads(args.manifest.read_text())
    args.output.mkdir(parents=True, exist_ok=True)
    args.environment = audit_environment(args)
    record_run(args, args.mode)
    with ThreadPoolExecutor(max_workers=3) as pool:
        work = [pool.submit(probe_project, args, project) for project in manifest["projects"] if args.language is None or project["language"] == args.language]
        for future in as_completed(work):
            future.result()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["prepare", "run", "probe"])
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=Path(__file__).with_name("oss-corpus.json"))
    parser.add_argument("--binary", type=lambda p: Path(p).resolve(), default=Path("target/release/context-creator").resolve())
    parser.add_argument("--probe", type=lambda p: Path(p).resolve(), default=Path("target/release/examples/audit_language_support").resolve())
    parser.add_argument("--mode", choices=["full", "sessions", "symbols"], default="full")
    parser.add_argument("--language", help="Optional exact family name for run/probe")
    parser.add_argument("--user-git-config", action="store_true", help="Include machine-specific global ignore rules")
    parser.add_argument("--output", type=Path, default=Path("/tmp/context-creator-oss-results"))
    args = parser.parse_args()
    if args.action == "prepare":
        prepare(args)
    elif args.action == "run":
        run(args)
    else:
        probe(args)


if __name__ == "__main__":
    main()
