from __future__ import annotations

import unittest

from ruby_bench_to_run import build_variant, group_benchmarks, split_subject


GIT = {"commit_sha": "local"}


class RubyBenchConversionTests(unittest.TestCase):
    def test_groups_both_subjects_of_a_case_with_boltffi_first(self) -> None:
        entries = [
            {"name": "uniffi_echo_i32", "loops": 8, "values_ns": [90.0, 110.0]},
            {"name": "boltffi_echo_i32", "loops": 8, "values_ns": [20.0, 30.0]},
            {"name": "boltffi_noop", "loops": 8, "values_ns": [10.0]},
        ]

        benchmarks = group_benchmarks(entries, git=GIT, rust_details=None, profile="release")

        self.assertEqual(["echo_i32", "noop"], [benchmark["descriptor"]["id"] for benchmark in benchmarks])
        self.assertEqual("ruby", benchmarks[0]["descriptor"]["language"])
        subjects = [variant["subject"]["attributes"]["subject_key"] for variant in benchmarks[0]["variants"]]
        self.assertEqual(["boltffi", "uniffi"], subjects)

    def test_metrics_describe_one_operation_from_the_raw_samples(self) -> None:
        variant = build_variant(
            entry={"name": "boltffi_add", "loops": 4, "values_ns": [30.0, 10.0, 20.0]},
            subject_prefix="boltffi",
            git=GIT,
            rust_details=None,
            profile="release",
        )

        metrics = variant["metrics"]
        self.assertEqual(20.0, metrics["value"])
        self.assertEqual(10.0, metrics["std_dev"])
        self.assertEqual((10.0, 30.0), (metrics["min"], metrics["max"]))
        self.assertEqual(20.0, metrics["percentiles"]["50.0"])
        self.assertEqual(12, variant["sampling"]["total_operations"])
        self.assertEqual("local", variant["subject"]["tool"]["git_sha"])

    def test_single_sample_has_no_standard_deviation(self) -> None:
        variant = build_variant(
            entry={"name": "uniffi_add", "loops": 1, "values_ns": [5.0]},
            subject_prefix="uniffi",
            git=GIT,
            rust_details=None,
            profile="release",
        )

        self.assertIsNone(variant["metrics"]["std_dev"])
        self.assertEqual(["uniffi"], variant["subject"]["build"]["features"])

    def test_rejects_a_benchmark_without_a_known_subject(self) -> None:
        with self.assertRaises(SystemExit):
            split_subject("magnus_add")


if __name__ == "__main__":
    unittest.main()
