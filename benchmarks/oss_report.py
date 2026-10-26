#!/usr/bin/env python3
"""Summarize raw OSS audit receipts without publishing upstream source bodies."""

import argparse
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import quote


def load(path):
    return json.loads(path.read_text()) if path.is_file() else None


def missing(expected, observed):
    return [name for name in expected if not any(
        value == name or value.endswith("::" + name) or value.endswith("." + name) or value.endswith("/" + name)
        for value in observed
    )]


def module_name(name):
    return re.sub(r"(?:/|\\|::|\.)+", "/", name).strip("/")


def compact_command(command):
    if command is None:
        return None
    return {key: command[key] for key in ["exit_code", "error", "seconds"] if key in command}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=Path(__file__).with_name("oss-corpus.json"))
    parser.add_argument("--oracles", type=Path, default=Path(__file__).with_name("oss-oracles.json"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout-confirmations", type=Path)
    args = parser.parse_args()
    manifest = load(args.manifest)
    oracles = load(args.oracles)["cases"]
    run = load(args.raw / "run.json") or load(args.raw / "run.cli.json") or {}
    records = []
    project_probes = []
    for project in manifest["projects"]:
        project_probes.append({
            "language": project["language"],
            **{mode: compact_command(load(args.raw / f'{project["cache_key"]}.{mode}.probe.json') or
                                     (load(args.raw / f'{project["cache_key"]}.probe.json') if mode == "full" else None))
               for mode in ["full", "sessions", "symbols"]},
        })
        for index, sample in enumerate(project["samples"]):
            key = project["language"].replace("+", "p").replace("#", "sharp").lower() + f"-{index}"
            oracle = oracles[key]
            cli = load(args.raw / f"{key}.cli.json")
            semantic = load(args.raw / f"{key}.semantic.json")
            symbols_only = load(args.raw / f"{key}.symbols.json")
            semantic_ok = semantic is not None and "error" not in semantic
            symbols_ok = symbols_only is not None and "error" not in symbols_only
            symbols = (semantic if semantic_ok else symbols_only if symbols_ok else {}).get("symbols", [])
            analysis = semantic if semantic_ok else {}
            observed_imports = []
            for item in analysis.get("imports", []):
                observed_imports.append(module_name(item["module"]))
                observed_imports.extend(module_name(item["module"] + "/" + name) for name in item["items"])
            comparison = {
                "missing_symbols": missing(oracle.get("expected_symbols", []), [item["name"] for item in symbols]) if semantic_ok or symbols_ok else None,
                "missing_calls": missing(oracle.get("expected_calls", []), [item["name"] for item in analysis.get("calls", [])]) if semantic_ok else None,
                "missing_types": missing(oracle.get("expected_types", []), [item["name"] for item in analysis.get("types", [])]) if semantic_ok else None,
                "missing_imports": [name for name in oracle.get("expected_imports", []) if not any(
                    observed == module_name(name) or observed.startswith(module_name(name) + "/") for observed in observed_imports
                )] if semantic_ok else None,
            }
            response = analysis.get("context", {}).get("response", {})
            record = {
                "id": key, "language": project["language"], "repository": project["repository"],
                "commit": project["commit"], "path": sample["path"], "source_sha256": sample["sha256"],
                "source_url": f'https://github.com/{project["repository"]}/blob/{project["commit"]}/{quote(sample["path"], safe="/")}',
                "source_lines": sample["lines"], "manual_check_id": key,
                "map": compact_command((cli or {}).get("map")),
                "combined_expansion": compact_command((cli or {}).get("expansion")),
                "semantic_completed": semantic_ok,
                "semantic_error": (semantic or {}).get("error"),
                "symbol_probe_completed": symbols_ok,
                "symbol_probe_seconds": (symbols_only or {}).get("seconds"),
                "observed": {
                    "symbols": [{name: symbol[name] for name in ["name", "kind", "line"]} for symbol in symbols],
                    "call_count": len(analysis.get("calls", [])) if semantic_ok else None,
                    "type_reference_count": len(analysis.get("types", [])) if semantic_ok else None,
                    "imports": [{"module": item["module"], "line": item["line"],
                                 "local_resolution": item["resolution"] is not None and item["resolution"].get("external") is False,
                                 "resolution_error": (item["resolution"] or {}).get("error")}
                                for item in analysis.get("imports", [])],
                    "parse_diagnostics": analysis.get("parse_errors") if semantic_ok else None,
                    "checked_private_visibility": [item for item in analysis.get("function_definitions", []) if item["name"] in oracle.get("expected_private_functions", [])],
                    "unexpected_parse_diagnostic": bool(analysis.get("parse_errors")) and oracle.get("expected_parse_error") is False if semantic_ok else None,
                },
                "comparison": comparison,
                "context": {name: analysis.get("context", {}).get(name) for name in ["exact_token_count", "within_budget", "invalid_source_snippets", "error"]},
                "context_selection": {"snippets": len(response.get("snippets", [])), "token_count": response.get("token_count"),
                                      "omitted_snippets": response.get("omitted_snippets"), "truncated_metadata": response.get("truncated_metadata")},
                "read_file": {name: analysis.get("read_file", {}).get(name) for name in ["page_matches_source", "page_within_budget", "error"]},
                "session": load(args.raw / f"{key}.sessions.json") if index == 0 else None,
            }
            records.append(record)
    all_context = [record for record in records if record["semantic_completed"]]
    summary = {
        "language_families": len(manifest["projects"]), "samples": len(records),
        "manual_source_reviews": len(oracles),
        "map_attempts": sum(r["map"] is not None for r in records),
        "map_successes": sum((r["map"] or {}).get("exit_code") == 0 for r in records),
        "combined_expansion_attempts": sum(r["combined_expansion"] is not None for r in records),
        "combined_expansion_successes": sum((r["combined_expansion"] or {}).get("exit_code") == 0 for r in records),
        "completed_semantic_cases": len(all_context),
        "exact_context_budgets": sum(r["context"]["exact_token_count"] is True and r["context"]["within_budget"] is True for r in all_context),
        "context_source_mismatches": sum(bool(r["context"]["invalid_source_snippets"]) for r in all_context),
        "source_page_successes": sum(r["read_file"]["page_matches_source"] is True for r in all_context),
        "samples_with_checked_symbol_omissions": sum(bool(r["comparison"]["missing_symbols"]) for r in records),
        "samples_with_checked_call_omissions": sum(bool(r["comparison"]["missing_calls"]) for r in records),
        "samples_with_checked_type_omissions": sum(bool(r["comparison"]["missing_types"]) for r in records),
        "samples_with_checked_import_omissions": sum(bool(r["comparison"]["missing_imports"]) for r in records),
        "completed_session_families": sum(r["session"] is not None and "error" not in r["session"] for r in records),
        "session_delivery_contract_passes": sum(r["session"] is not None and (r["session"].get("no_repeat_without_reset") is True or r["session"].get("repeated_content_snippets") == 0) for r in records),
        "explicit_session_resets_confirmed": sum((r["session"] or {}).get("unchanged_was_reset") is True for r in records),
        "session_revision_checks_passed": sum(all((r["session"] or {}).get(key) is True for key in ["unchanged_analysis_reused", "edit_revision_changed", "stale_revision_rejected"]) for r in records),
    }
    isolated_sql = []
    for path in sorted((args.raw / "isolated-sql").glob("*.semantic.json")):
        result = load(path)
        isolated_sql.append({"path": result["path"], "read_file": {name: result.get("read_file", {}).get(name) for name in ["error", "page_matches_source", "page_within_budget"]}})
    output = {
        "schema_version": 1, "tool_source_commit": run.get("source_commit"),
        "random_seed": manifest["seed"], "manifest_sha256": hashlib.sha256(args.manifest.read_bytes()).hexdigest(),
        "method": "Three files sampled once per family; manual source expectations compared with actual CLI/crate/read-helper output. Missing expectations are individual observations, not complete recall estimates.",
        "limits": {"cli_timeout_seconds": 90, "full_semantic_project_timeout_seconds": 420,
                   "parallel_projects": 3, "context_tokens": 8192,
                   "baseline_git_config": run.get("global_git_config", "unrecorded"),
                   "initial_session_metric": "Discarded the all-content-absent metric: unchanged requests may deliver previously omitted snippets. Supplemental sessions compare IDs and content hashes."},
        "summary": summary, "run": run, "project_probes": project_probes,
        "neutral_git_config_sql_checks": isolated_sql,
        "standalone_timeout_confirmations": [{name: row[name] for name in ["key", "feature", "exit_code", "error", "seconds"] if name in row} for row in (load(args.timeout_confirmations) if args.timeout_confirmations else [])],
        "records": records,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
