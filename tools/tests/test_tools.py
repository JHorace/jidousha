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
serve_web = load_tool("serve-web")
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


class ExampleDiscoveryTest(unittest.TestCase):
    METADATA = {
        "workspace_members": ["core-id", "facade-id"],
        "packages": [
            {
                "id": "core-id",
                "name": "jidousha-core",
                "targets": [
                    {"name": "jidousha-core", "kind": ["lib"]},
                    {"name": "homing", "kind": ["example"]},
                ],
            },
            {
                "id": "facade-id",
                "name": "jidousha",
                "targets": [{"name": "quickstart", "kind": ["example"]}],
            },
            {
                "id": "dependency-id",
                "name": "some-dependency",
                "targets": [{"name": "demo", "kind": ["example"]}],
            },
        ],
    }

    def test_every_workspace_example_is_discovered(self):
        found = test_wrapper.parse_example_targets(self.METADATA)
        self.assertEqual(found, [("jidousha", "quickstart"), ("jidousha-core", "homing")])

    def test_examples_belonging_to_dependencies_are_not_run(self):
        found = test_wrapper.parse_example_targets(self.METADATA)
        self.assertNotIn(("some-dependency", "demo"), found)

    def test_a_workspace_with_no_examples_discovers_none(self):
        self.assertEqual(test_wrapper.parse_example_targets({}), [])

    def test_an_ordinary_example_is_run(self):
        name, command = test_wrapper.example_phase("jidousha-core", "homing")
        self.assertEqual(name, "example:homing")
        self.assertIn("run", command)

    def test_a_windowed_example_is_built_and_not_run(self):
        # It would open a window and wait for a person; a headless runner has
        # no display and would fail for a reason that says nothing about the
        # code. Building still catches every compile error.
        example = sorted(test_wrapper.WINDOWED_EXAMPLES)[0]
        name, command = test_wrapper.example_phase("jidousha-platform", example)
        self.assertEqual(name, f"example-build:{example}")
        self.assertIn("build", command)
        self.assertNotIn("run", command)

    def test_the_windowed_list_names_examples_that_exist(self):
        # A stale name here would silently start running a windowed example, or
        # keep skipping one that was deleted.
        root = Path(__file__).resolve().parents[2]
        existing = {path.stem for path in root.glob("crates/*/examples/*.rs")}
        self.assertTrue(
            test_wrapper.WINDOWED_EXAMPLES <= existing,
            f"unknown windowed examples: {test_wrapper.WINDOWED_EXAMPLES - existing}",
        )


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


def png_bytes(width, height, rows, filter_kind=0):
    """A minimal 8-bit RGB PNG, for the decoder tests below.

    Written by hand for the same reason the decoder is: making one is fifteen
    lines, and the alternative is a dependency in a test for a tool.
    """
    import struct
    import zlib

    raw = bytearray()
    previous = [0] * (width * 3)
    for row in rows:
        flat = [channel for pixel in row for channel in pixel]
        raw.append(filter_kind)
        if filter_kind == 0:
            raw.extend(flat)
        elif filter_kind == 2:  # Up
            raw.extend((value - up) & 0xFF for value, up in zip(flat, previous))
        else:
            raise ValueError("only None and Up filters are generated here")
        previous = flat

    def chunk(kind, body):
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw)))
        + chunk(b"IEND", b"")
    )


class ServeWebTest(unittest.TestCase):
    """The web harness: the version check, and the screenshot decoder."""

    def test_an_unfiltered_png_decodes_to_its_pixels(self):
        rows = [[(255, 0, 0), (0, 255, 0)], [(0, 0, 255), (10, 20, 30)]]
        found = serve_web.decode_png(png_bytes(2, 2, rows))
        self.assertIsNotNone(found)
        width, height, pixels = found
        self.assertEqual((width, height), (2, 2))
        self.assertEqual(pixels, [pixel for row in rows for pixel in row])

    def test_a_filtered_png_decodes_to_its_pixels(self):
        # Chromium's screenshots are filtered; a decoder that only handled the
        # None filter would work on the test above and fail on every real one.
        rows = [[(9, 9, 9), (40, 50, 60)], [(11, 12, 13), (200, 100, 50)]]
        found = serve_web.decode_png(png_bytes(2, 2, rows, filter_kind=2))
        self.assertIsNotNone(found)
        self.assertEqual(found[2], [pixel for row in rows for pixel in row])

    def test_bytes_that_are_not_a_png_are_refused(self):
        self.assertIsNone(serve_web.decode_png(b"this is not a png"))

    def test_a_blank_canvas_reads_as_nothing_drawn(self):
        # The exact failure this check exists for: the page loads, the engine
        # runs, and the canvas stays the page's own background color.
        background = [[(0x10, 0x10, 0x14)] * 8 for _ in range(8)]
        drawn, detail = serve_web.canvas_is_drawn(png_bytes(8, 8, background))
        self.assertFalse(drawn, detail)

    def test_a_painted_canvas_reads_as_drawn(self):
        painted = [[(230, 194, 100)] * 8 for _ in range(8)]
        drawn, detail = serve_web.canvas_is_drawn(png_bytes(8, 8, painted))
        self.assertTrue(drawn, detail)

    def test_the_wasm_bindgen_version_is_read_from_the_lockfile(self):
        # The CLI and the crate generate two halves of one interface, so this
        # is what stops a skew from becoming a runtime mystery.
        version = serve_web.locked_wasm_bindgen_version()
        self.assertIsNotNone(version, "Cargo.lock should pin wasm-bindgen")
        self.assertRegex(version, r"^\d+\.\d+\.\d+$")
