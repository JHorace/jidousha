"""Behavioral tests for the tools/ scripts (agent-practices §5.2).

Run by `tools/test` as its first phase: if the wrapper is broken, its report of
the engine's tests cannot be trusted either.

The scripts are extensionless executables, so they are loaded by path rather
than imported by name.
"""

import importlib.machinery
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent


def load_tool(script_name):
    """Import an extensionless tools/ script as a module."""
    path = REPO_ROOT / "tools" / script_name
    module_name = "jidousha_tool_" + script_name.replace("-", "_")
    loader = importlib.machinery.SourceFileLoader(module_name, str(path))
    spec = importlib.util.spec_from_loader(module_name, loader)
    module = importlib.util.module_from_spec(spec)
    # Registered before execution: `dataclass` resolves annotations through
    # sys.modules, and fails on a module that is not there yet.
    sys.modules[module_name] = module
    loader.exec_module(module)
    return module


doctor = load_tool("doctor")
test_wrapper = load_tool("test")
check_claude_md = load_tool("check-claude-md")
dep_count = load_tool("dep-count")


class DoctorVerdictTest(unittest.TestCase):
    def test_verdict_is_env_ok_when_every_check_passes(self):
        checks = [doctor.Check("a", doctor.OK, "fine"), doctor.Check("b", doctor.INFO, "noted")]
        self.assertEqual(doctor.verdict(checks), ("ENV_OK", 0))

    def test_verdict_names_the_exact_fix_command_when_a_check_is_fixable(self):
        checks = [doctor.Check("wasm-target", doctor.FIXABLE, "missing", fix="rustup target add x")]
        line, code = doctor.verdict(checks)
        self.assertEqual(line, "ENV_FIXABLE: rustup target add x")
        self.assertEqual(code, 1)

    def test_verdict_joins_every_fix_command_when_several_checks_are_fixable(self):
        checks = [
            doctor.Check("a", doctor.FIXABLE, "missing", fix="cmd one"),
            doctor.Check("b", doctor.FIXABLE, "missing", fix="cmd two"),
        ]
        self.assertEqual(doctor.verdict(checks)[0], "ENV_FIXABLE: cmd one && cmd two")

    def test_verdict_reports_env_broken_ahead_of_any_fixable_check(self):
        checks = [
            doctor.Check("a", doctor.FIXABLE, "missing", fix="cmd"),
            doctor.Check("disk", doctor.BROKEN, "0.2 GiB free"),
        ]
        line, code = doctor.verdict(checks)
        self.assertEqual(line, "ENV_BROKEN: disk: 0.2 GiB free")
        self.assertEqual(code, 2)


class DoctorToolchainTest(unittest.TestCase):
    def test_pinned_channel_is_read_from_rust_toolchain_toml(self):
        toml = '[toolchain]\nchannel = "1.94.1"\ncomponents = ["clippy"]\n'
        self.assertEqual(doctor.pinned_channel(toml), "1.94.1")

    def test_toolchain_check_passes_when_active_rustc_matches_the_pin(self):
        check = doctor.compare_toolchain("1.94.1", "rustc 1.94.1 (e408947bf 2026-03-25)")
        self.assertEqual(check.status, doctor.OK)

    def test_toolchain_check_is_fixable_when_active_rustc_differs_from_the_pin(self):
        check = doctor.compare_toolchain("1.94.1", "rustc 1.90.0 (aaaaaaaaa 2025-01-01)")
        self.assertEqual(check.status, doctor.FIXABLE)
        self.assertEqual(check.fix, "rustup toolchain install 1.94.1")

    def test_toolchain_check_looks_past_rustup_install_chatter_for_the_version(self):
        # Verbatim from a fresh CI runner materializing the pinned toolchain.
        output = (
            "info: syncing channel updates for 1.94.1-x86_64-unknown-linux-gnu\n"
            "info: latest update on 2026-03-26 for version 1.94.1 (e408947bf 2026-03-25)\n"
            "info: downloading 6 components\n"
            "rustc 1.94.1 (e408947bf 2026-03-25)"
        )
        check = doctor.compare_toolchain("1.94.1", output)
        self.assertEqual(check.status, doctor.OK)

    def test_toolchain_check_accepts_a_named_channel_without_comparing_versions(self):
        check = doctor.compare_toolchain("stable", "rustc 1.94.1 (e408947bf 2026-03-25)")
        self.assertEqual(check.status, doctor.OK)

    def test_toolchain_check_is_broken_when_the_repo_pin_is_missing(self):
        check = doctor.compare_toolchain(None, "rustc 1.94.1 (e408947bf 2026-03-25)")
        self.assertEqual(check.status, doctor.BROKEN)

    def test_every_fixable_check_carries_a_command_to_run(self):
        # CONTRACT from tools/doctor: fix is non-empty exactly when status is FIXABLE.
        for probe in doctor.CHECKS:
            check = probe()
            with self.subTest(check=check.name):
                self.assertEqual(check.status == doctor.FIXABLE, bool(check.fix))


SAMPLE_OUTPUT = """
running 2 tests
test entity_ids_are_generational ... ok
test despawn_bumps_the_generation ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 2 tests
test iteration_order_is_stable ... FAILED
test queries_skip_dead_entities ... ok

test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s
"""


class TestOutputParsingTest(unittest.TestCase):
    def test_counts_are_summed_across_every_test_binary(self):
        parsed = test_wrapper.parse_test_output(SAMPLE_OUTPUT)
        self.assertEqual((parsed["passed"], parsed["failed"], parsed["ignored"]), (3, 1, 1))

    def test_failed_test_names_are_collected(self):
        parsed = test_wrapper.parse_test_output(SAMPLE_OUTPUT)
        self.assertEqual(parsed["failed_tests"], ["iteration_order_is_stable"])

    def test_empty_output_parses_as_zero_tests_rather_than_crashing(self):
        parsed = test_wrapper.parse_test_output("")
        self.assertEqual(parsed, {"passed": 0, "failed": 0, "ignored": 0, "failed_tests": []})

    def test_the_tools_own_unittest_results_land_in_the_same_totals(self):
        parsed = test_wrapper.parse_selftest_output(
            "FAIL: test_counts_are_summed (test_tools.TestOutputParsingTest)\n"
            "----\nRan 4 tests in 0.01s\n\nFAILED (failures=1)\n"
        )
        self.assertEqual(parsed["passed"], 3)
        self.assertEqual(parsed["failed"], 1)
        self.assertEqual(parsed["failed_tests"], ["test_counts_are_summed"])


def report(status="fail", failed_tests=(), failing_phase="test", tail=("error: something broke",)):
    return {
        "status": status,
        "failed_tests": list(failed_tests),
        "phases": [
            {"name": "build", "status": "ok", "output_tail": []},
            {"name": failing_phase, "status": "failed", "output_tail": list(tail)},
        ],
    }


class FailureStreakTest(unittest.TestCase):
    def test_the_same_failure_produces_the_same_fingerprint(self):
        first = test_wrapper.failure_fingerprint(report(failed_tests=["a", "b"]))
        second = test_wrapper.failure_fingerprint(report(failed_tests=["b", "a"]))
        self.assertEqual(first, second)

    def test_a_different_failing_test_produces_a_different_fingerprint(self):
        first = test_wrapper.failure_fingerprint(report(failed_tests=["a"]))
        second = test_wrapper.failure_fingerprint(report(failed_tests=["c"]))
        self.assertNotEqual(first, second)

    def test_a_different_error_with_no_named_test_produces_a_different_fingerprint(self):
        first = test_wrapper.failure_fingerprint(report(tail=("error[E0425]: cannot find `foo`",)))
        second = test_wrapper.failure_fingerprint(report(tail=("error: linker `cc` not found",)))
        self.assertNotEqual(first, second)

    def test_the_same_error_with_different_timings_keeps_one_fingerprint(self):
        first = test_wrapper.failure_fingerprint(report(tail=("finished in 0.01s",)))
        second = test_wrapper.failure_fingerprint(report(tail=("finished in 9.42s",)))
        self.assertEqual(first, second)

    def test_a_failure_in_a_different_phase_produces_a_different_fingerprint(self):
        first = test_wrapper.failure_fingerprint(report(failing_phase="test"))
        second = test_wrapper.failure_fingerprint(report(failing_phase="doc-test"))
        self.assertNotEqual(first, second)

    def test_repeating_the_same_failure_increments_the_streak(self):
        with tempfile.TemporaryDirectory() as tmp:
            original = test_wrapper.STREAK_PATH
            test_wrapper.STREAK_PATH = Path(tmp) / "failure-streak.json"
            try:
                self.assertEqual(test_wrapper.update_streak("abc"), 1)
                self.assertEqual(test_wrapper.update_streak("abc"), 2)
                self.assertEqual(test_wrapper.update_streak("def"), 1)
                stored = json.loads(test_wrapper.STREAK_PATH.read_text(encoding="utf-8"))
                self.assertEqual(stored, {"fingerprint": "def", "count": 1})
            finally:
                test_wrapper.STREAK_PATH = original

    def test_a_passing_run_clears_the_streak(self):
        with tempfile.TemporaryDirectory() as tmp:
            original = test_wrapper.STREAK_PATH
            test_wrapper.STREAK_PATH = Path(tmp) / "failure-streak.json"
            try:
                test_wrapper.update_streak("abc")
                test_wrapper.clear_streak()
                self.assertFalse(test_wrapper.STREAK_PATH.exists())
            finally:
                test_wrapper.STREAK_PATH = original


class ClaudeMdSizeTest(unittest.TestCase):
    def test_a_file_within_the_cap_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "CLAUDE.md"
            path.write_text("line\n" * 10, encoding="utf-8")
            self.assertEqual(check_claude_md.check_line_count(path, 150), (True, 10))

    def test_a_file_over_the_cap_fails_and_reports_its_length(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "CLAUDE.md"
            path.write_text("line\n" * 151, encoding="utf-8")
            self.assertEqual(check_claude_md.check_line_count(path, 150), (False, 151))

    def test_the_committed_claude_md_is_within_its_cap(self):
        within_cap, count = check_claude_md.check_line_count(
            check_claude_md.CLAUDE_MD, check_claude_md.LINE_CAP
        )
        self.assertTrue(within_cap, f"CLAUDE.md is {count} lines")


class DepCountTest(unittest.TestCase):
    def test_workspace_members_are_not_counted_as_dependencies(self):
        metadata = {
            "workspace_members": ["jidousha-core 0.1.0 (path+file:///w)"],
            "packages": [
                {"id": "jidousha-core 0.1.0 (path+file:///w)", "name": "jidousha-core", "version": "0.1.0"},
                {"id": "glam 0.30.0 (registry+…)", "name": "glam", "version": "0.30.0"},
            ],
        }
        total, names = dep_count.count_dependencies(metadata)
        self.assertEqual(total, 1)
        self.assertEqual(names, ["glam 0.30.0"])


if __name__ == "__main__":
    unittest.main()
