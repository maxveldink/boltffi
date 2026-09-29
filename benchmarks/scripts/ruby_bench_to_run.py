#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any

from benchmark_schema import (
    REPO_ROOT,
    artifacts,
    build_run_id,
    ci_context,
    collector_context,
    crate_version,
    git_context,
    host_context,
    infer_descriptor,
    rust_toolchain,
    sha256_file,
    utc_now,
)


BOLTFFI_VERSION = crate_version(REPO_ROOT / "boltffi/Cargo.toml")
UNIFFI_VERSION = "0.31.1"
SUBJECT_ORDER = ["boltffi", "uniffi"]
SUBJECT_CONFIG = {
    "boltffi": {
        "tool": {
            "name": "boltffi",
            "version": BOLTFFI_VERSION,
            "git_sha": "repository_head",
            "crate_version": BOLTFFI_VERSION,
        },
        "ffi": {
            "bridge": "boltffi",
            "transport": "ruby_capi",
            "ownership_model": None,
            "attributes": {
                "host_language": "ruby",
                "binding_runtime": "c_extension",
            },
        },
        "attributes": {
            "subject_key": "boltffi",
            "binding_module": "BenchBoltFFI",
        },
    },
    "uniffi": {
        "tool": {
            "name": "uniffi",
            "version": UNIFFI_VERSION,
            "git_sha": None,
            "crate_version": None,
        },
        "ffi": {
            "bridge": "uniffi",
            "transport": "ffi_gem",
            "ownership_model": None,
            "attributes": {
                "host_language": "ruby",
                "binding_runtime": "ffi_gem",
            },
        },
        "attributes": {
            "subject_key": "uniffi",
            "binding_module": "Demo",
        },
    },
}


def parse_args() -> argparse.Namespace:
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument("--results", type=Path, required=True)
    argument_parser.add_argument("--output", type=Path, required=True)
    argument_parser.add_argument("--profile", default="release")
    argument_parser.add_argument("--runner-command")
    return argument_parser.parse_args()


def main() -> None:
    args = parse_args()
    results = json.loads(args.results.read_text())
    if not results.get("benchmarks"):
        raise SystemExit(f"no Ruby benchmarks found in {args.results}")

    git = git_context()
    collected_at = utc_now()
    rust_details = rust_toolchain()
    ruby = results["ruby"]
    suite_name = "ruby-bench"
    run = {
        "schema_version": "benchmark_run_v1",
        "run_id": build_run_id(f"{suite_name}-yjit" if ruby["yjit"] else suite_name, collected_at, git["commit_sha"]),
        "collected_at": collected_at,
        "provenance": {
            "repository": git,
            "collector": collector_context(
                invocation=args.runner_command or f"ruby_bench_to_run.py --results {args.results}"
            ),
            "artifacts": artifacts([args.results]),
        },
        "environment": {
            "host": host_context(),
            "toolchains": {
                "rust": rust_details,
                "swift": None,
                "kotlin": None,
                "java": None,
                "node": None,
                "wasm": None,
            },
            "runtime": {
                "engine": "cruby" if ruby["engine"] == "ruby" else ruby["engine"],
                "version": ruby["version"],
                "platform": "native",
                "attributes": {
                    "ruby_description": ruby["description"],
                    "ruby_executable": ruby["executable"],
                    "yjit": ruby["yjit"],
                    "runner_command": args.runner_command,
                },
            },
            "ci": ci_context(),
        },
        "suite": {
            "name": suite_name,
            "harness": "ruby-bench",
            "platform": "native",
            "language": "ruby",
            "profile": args.profile,
            "tags": ["yjit"] if ruby["yjit"] else [],
            "attributes": {
                "results_sha256": sha256_file(args.results),
                "benchmark_count": len(results["benchmarks"]),
                "settings": results["settings"],
            },
        },
        "benchmarks": group_benchmarks(
            results["benchmarks"],
            git=git,
            rust_details=rust_details,
            profile=args.profile,
        ),
        "notes": [],
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(run, indent=2) + "\n")


def group_benchmarks(
    entries: list[dict[str, Any]],
    *,
    git: dict[str, Any],
    rust_details: dict[str, Any] | None,
    profile: str,
) -> list[dict[str, Any]]:
    grouped_cases: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for entry in entries:
        subject_prefix, case_name = split_subject(entry["name"])
        grouped_cases[case_name].append(
            build_variant(
                entry=entry,
                subject_prefix=subject_prefix,
                git=git,
                rust_details=rust_details,
                profile=profile,
            )
        )

    return [
        {
            "descriptor": infer_descriptor(case_name, "native", "ruby"),
            "variants": sorted(
                variants,
                key=lambda variant: SUBJECT_ORDER.index(variant["subject"]["attributes"]["subject_key"]),
            ),
            "notes": [],
        }
        for case_name, variants in sorted(grouped_cases.items())
    ]


def split_subject(benchmark_name: str) -> tuple[str, str]:
    for subject_prefix in SUBJECT_CONFIG:
        prefixed_name = f"{subject_prefix}_"
        if benchmark_name.startswith(prefixed_name):
            return subject_prefix, benchmark_name.removeprefix(prefixed_name)

    raise SystemExit(f"unsupported Ruby benchmark name {benchmark_name!r}")


def build_variant(
    *,
    entry: dict[str, Any],
    subject_prefix: str,
    git: dict[str, Any],
    rust_details: dict[str, Any] | None,
    profile: str,
) -> dict[str, Any]:
    values_ns = sorted(entry["values_ns"])
    loops = entry.get("loops")

    tool_identity = dict(SUBJECT_CONFIG[subject_prefix]["tool"])
    if tool_identity["git_sha"] == "repository_head":
        tool_identity["git_sha"] = git["commit_sha"]

    return {
        "subject": {
            "tool": tool_identity,
            "build": {
                "compiler_name": "rustc",
                "compiler_version": rust_details["rustc_version"] if rust_details else None,
                "target": rust_details["target_triple"] if rust_details else None,
                "profile": profile,
                "optimization": "release" if profile == "release" else "debug",
                "features": ["uniffi"] if subject_prefix == "uniffi" else [],
                "flags": ["--release"] if profile == "release" else [],
            },
            "ffi": SUBJECT_CONFIG[subject_prefix]["ffi"],
            "attributes": SUBJECT_CONFIG[subject_prefix]["attributes"],
        },
        "metrics": {
            "unit": "ns_per_op",
            "estimator": "mean",
            "value": statistics.fmean(values_ns),
            "std_dev": statistics.stdev(values_ns) if len(values_ns) > 1 else None,
            "min": values_ns[0],
            "max": values_ns[-1],
            "percentiles": {
                "50.0": percentile(values_ns, 0.5),
                "90.0": percentile(values_ns, 0.9),
                "95.0": percentile(values_ns, 0.95),
                "99.0": percentile(values_ns, 0.99),
            },
        },
        "sampling": {
            "warmup_iterations": None,
            "measurement_iterations": len(values_ns),
            "sample_count": len(values_ns),
            "total_operations": loops * len(values_ns) if isinstance(loops, int) else None,
        },
        "notes": [],
    }


def percentile(sorted_values: list[float], ratio: float) -> float:
    raw_index = (len(sorted_values) - 1) * ratio
    lower_index = int(raw_index)
    upper_index = min(lower_index + 1, len(sorted_values) - 1)
    weight = raw_index - lower_index
    return sorted_values[lower_index] * (1 - weight) + sorted_values[upper_index] * weight


if __name__ == "__main__":
    main()
