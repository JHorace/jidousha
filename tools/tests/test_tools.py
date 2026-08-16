"""Behavioral tests for the tools/ scripts (agent-practices §5.2).

Run by `tools/test` as its first phase: if the wrapper is broken, its report of
the engine's tests cannot be trusted either.

The scripts are extensionless executables, so they are loaded by path rather
than imported by name.
"""

import contextlib
import importlib.machinery
import importlib.util
import io
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
verify = load_tool("verify")
check_assets = load_tool("check-assets")


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


class DoctorGpuTest(unittest.TestCase):
    def test_installed_vulkan_drivers_are_listed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "lvp_icd.json").write_text("{}")
            (root / "radeon_icd.json").write_text("{}")
            (root / "notes.txt").write_text("not a driver")
            self.assertEqual(
                doctor.vulkan_drivers((str(root),)),
                ["lvp_icd.json", "radeon_icd.json"],
            )

    def test_a_directory_that_does_not_exist_is_not_an_error(self):
        # The common case on a machine with no graphics stack at all, and the
        # one where doctor must report rather than crash.
        self.assertEqual(doctor.vulkan_drivers(("/no/such/place",)), [])

    def test_the_gpu_check_never_blocks_a_run(self):
        # A machine with no GPU runs every other test in the suite. Marking this
        # FIXABLE or BROKEN would make doctor cry wolf, and then be ignored when
        # it matters (agent-practices §6.1).
        self.assertEqual(doctor.check_gpu().status, doctor.INFO)
        self.assertEqual(doctor.check_gpu().fix, "")


class CheckAssetsTest(unittest.TestCase):
    FILE_BACKED = 'const ASSET_ROOT: &str = "art";\nAssets::new(FileSource::new(ASSET_ROOT));\n'
    MEMORY_BACKED = 'let mut source = MemorySource::new();\nassets.load_texture("nowhere.png");\n'

    def test_a_file_that_loads_from_disk_is_checked(self):
        self.assertTrue(check_assets.builds_file_source(self.FILE_BACKED))

    def test_a_file_that_only_scripts_its_own_content_is_not_checked(self):
        # `examples/loading_gate.rs` names paths that deliberately do not exist.
        # Reporting them would make this check's first act a false alarm.
        self.assertFalse(check_assets.builds_file_source(self.MEMORY_BACKED))

    def test_the_crate_that_defines_the_seam_is_not_checked(self):
        # `files.rs` and `lib.rs` mention `FileSource::new` and `asset_source`
        # because they *are* those things; their doc examples name paths that
        # were never meant to exist.
        text = "pub struct FileSource {}\nAssets::new(FileSource::new(root));\n"
        self.assertTrue(check_assets.builds_file_source(text))
        self.assertTrue(check_assets.defines_the_source(text))

    def test_the_asset_root_is_read_from_the_constant_the_store_was_given(self):
        self.assertEqual(check_assets.asset_root_of(self.FILE_BACKED), "art")

    def test_an_asset_root_passed_as_a_literal_is_read_too(self):
        self.assertEqual(
            check_assets.asset_root_of('jidousha_platform::asset_source("stuff")'), "stuff"
        )

    def test_a_root_that_cannot_be_resolved_is_reported_rather_than_guessed(self):
        # Guessing "assets" here would check the wrong directory and pass.
        self.assertIsNone(check_assets.asset_root_of("Assets::new(FileSource::new(computed))"))

    def test_literal_loads_are_found_with_their_line_and_kind(self):
        text = 'x\nlet a = assets.load_texture("sprites/hero.png");\nlet b = assets.load_bytes("level.dat");\n'
        self.assertEqual(
            check_assets.literal_loads(text),
            [(2, "texture", "sprites/hero.png"), (3, "bytes", "level.dat")],
        )

    def test_a_load_marked_deliberately_missing_is_not_a_broken_reference(self):
        text = (
            "// check-assets: deliberately missing\n"
            'let missing = assets.load_texture("nothing_here.png");\n'
        )
        self.assertEqual(check_assets.literal_loads(text), [])

    def test_a_marker_does_not_leak_past_the_comment_block_it_is_in(self):
        # Otherwise one marker near the top of a file would silence every load
        # below it, and the check would quietly stop checking.
        text = (
            "// check-assets: deliberately missing\n"
            'let a = assets.load_texture("gone.png");\n'
            "let unrelated = 1;\n"
            'let b = assets.load_texture("real.png");\n'
        )
        self.assertEqual(check_assets.literal_loads(text), [(4, "texture", "real.png")])

    def test_a_computed_path_is_reported_because_it_cannot_be_checked(self):
        text = 'let a = assets.load_texture(&format!("levels/{n}/bg.png"));\n'
        self.assertEqual(len(check_assets.computed_loads(text)), 1)

    def test_a_computed_path_the_convention_sanctions_is_accepted(self):
        text = (
            "// check-assets: computed path — one directory per numbered level\n"
            'let a = assets.load_texture(&format!("levels/{n}/bg.png"));\n'
        )
        self.assertEqual(check_assets.computed_loads(text), [])

    def test_a_path_that_exists_resolves(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "sprites").mkdir()
            (root / "sprites" / "hero.png").write_bytes(b"x")
            self.assertEqual(
                check_assets.resolve_case_strict(root, "sprites/hero.png"), (True, None)
            )

    def test_a_path_that_differs_only_in_case_names_the_file_on_disk(self):
        # The single most valuable thing this check says: it turns "works on my
        # machine, 404s on the server" into a rename.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "sprites").mkdir()
            (root / "sprites" / "hero.png").write_bytes(b"x")
            found, near = check_assets.resolve_case_strict(root, "sprites/Hero.png")
            self.assertFalse(found)
            self.assertEqual(near, "sprites/hero.png")

    def test_a_wrongly_cased_directory_is_caught_too(self):
        # Walked component by component precisely so this case is not missed.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "sprites").mkdir()
            (root / "sprites" / "hero.png").write_bytes(b"x")
            found, near = check_assets.resolve_case_strict(root, "Sprites/hero.png")
            self.assertFalse(found)
            self.assertEqual(near, "sprites")

    def test_a_path_that_is_simply_absent_has_no_near_miss(self):
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(
                check_assets.resolve_case_strict(Path(directory), "nowhere.png"), (False, None)
            )

    def test_a_directory_is_not_a_file(self):
        # `load_texture("sprites")` names something that exists and cannot be
        # loaded; the engine reports it as unreadable at runtime.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "sprites").mkdir()
            self.assertEqual(check_assets.resolve_case_strict(root, "sprites"), (False, None))

    def test_a_broken_reference_is_reported_in_the_engines_message_shape(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "art").mkdir()
            source = repo / "game.rs"
            source.write_text(
                'const ASSET_ROOT: &str = "art";\n'
                "fn main() { Assets::new(FileSource::new(ASSET_ROOT)); }\n"
                'fn go() { assets.load_texture("gone.png"); }\n'
            )
            problems = check_assets.check_file(repo, source)
            self.assertEqual(len(problems), 1)
            text = problems[0]
            self.assertTrue(text.startswith("[jidousha] asset failed: 'gone.png'"), text)
            self.assertIn("requested by: load_texture at game.rs:3", text)
            self.assertIn("likely cause:", text)
            self.assertIn("fix:", text)

    def test_the_case_mismatch_wording_still_matches_the_engines_own(self):
        # The cost of a Python check reporting a Rust taxonomy: the case-mismatch
        # sentence exists in both. This is the drift alarm — reword the Rust
        # message and this fails, pointing at the copy in tools/check-assets
        # rather than letting the two quietly diverge (assets.md §6).
        root = Path(__file__).resolve().parents[2]
        rust = (root / "crates/jidousha-assets/src/payload.rs").read_text(encoding="utf-8")
        shared = "rename the file or the path so they match exactly"
        self.assertIn(shared, rust, "the engine's message changed")
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "art").mkdir()
            (repo / "art" / "hero.png").write_bytes(b"x")
            source = repo / "game.rs"
            source.write_text(
                'const ASSET_ROOT: &str = "art";\n'
                "fn main() { Assets::new(FileSource::new(ASSET_ROOT)); }\n"
                'fn go() { assets.load_texture("Hero.png"); }\n'
            )
            problems = check_assets.check_file(repo, source)
            self.assertEqual(len(problems), 1)
            self.assertIn(shared, problems[0], "the check's copy changed")

    def _run_main(self, repo):
        """Run the script end to end against `repo`, returning (code, stderr)."""
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = check_assets.main(["tools/check-assets", "--root", str(repo)])
        return code, err.getvalue()

    def _repo_with(self, load: str):
        directory = tempfile.TemporaryDirectory()
        repo = Path(directory.name)
        (repo / "art").mkdir()
        (repo / "art" / "hero.png").write_bytes(b"x")
        crate = repo / "crates" / "game" / "examples"
        crate.mkdir(parents=True)
        (crate / "game.rs").write_text(
            'const ASSET_ROOT: &str = "art";\n'
            "fn main() { Assets::new(FileSource::new(ASSET_ROOT)); }\n"
            f"fn go() {{ {load} }}\n"
        )
        return directory, repo

    def test_a_run_over_a_sound_tree_exits_zero(self):
        holder, repo = self._repo_with('assets.load_texture("hero.png");')
        with holder:
            code, _ = self._run_main(repo)
            self.assertEqual(code, 0)

    def test_a_broken_reference_makes_the_whole_run_fail(self):
        # The wiring between finding a problem and saying so with a non-zero
        # exit. Without this, every check above could pass while the script
        # cheerfully returned 0 and CI stayed green.
        holder, repo = self._repo_with('assets.load_texture("gone.png");')
        with holder:
            code, err = self._run_main(repo)
            self.assertEqual(code, 1)
            self.assertIn("asset failed: 'gone.png'", err)
            self.assertIn("1 broken asset reference(s)", err)

    def test_a_run_with_nothing_to_check_is_a_tooling_fault(self):
        # An empty tree means the check was pointed somewhere wrong, and
        # reporting "all clear" would be the most misleading answer available.
        with tempfile.TemporaryDirectory() as directory:
            code, err = self._run_main(Path(directory))
            self.assertEqual(code, 2)
            self.assertIn("found no Rust sources", err)

    def test_the_committed_tree_has_no_broken_asset_references(self):
        # The check checking itself, on the real repository — so a rename that
        # breaks a path fails here as well as in CI.
        root = Path(__file__).resolve().parents[2]
        problems = []
        for source in check_assets.rust_sources(root):
            problems += check_assets.check_file(root, source)
        self.assertEqual(problems, [])


class DoctorAssetsTest(unittest.TestCase):
    def test_a_readable_asset_root_passes(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "assets").mkdir()
            (repo / "assets" / "hero.png").write_bytes(b"x")
            self.assertEqual(doctor.check_assets(repo).status, doctor.OK)

    def test_a_missing_asset_root_is_broken_rather_than_fixable(self):
        # Nothing an agent can run creates a directory of art, and a windowed
        # example opening to a screen of magenta is worth pre-empting.
        with tempfile.TemporaryDirectory() as directory:
            check = doctor.check_assets(Path(directory))
            self.assertEqual(check.status, doctor.BROKEN)
            self.assertEqual(check.fix, "", "BROKEN checks carry no fix command")

    def test_an_empty_asset_root_is_only_noted(self):
        # A repository may legitimately have no art yet; the engine draws the
        # placeholder rather than failing.
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "assets").mkdir()
            self.assertEqual(doctor.check_assets(repo).status, doctor.INFO)


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
        # Both example layouts cargo accepts: `examples/name.rs`, and
        # `examples/name/main.rs` for one big enough to be worth splitting.
        existing = {path.stem for path in root.glob("crates/*/examples/*.rs")}
        existing |= {path.parent.name for path in root.glob("crates/*/examples/*/main.rs")}
        self.assertTrue(
            test_wrapper.WINDOWED_EXAMPLES <= existing,
            f"unknown windowed examples: {test_wrapper.WINDOWED_EXAMPLES - existing}",
        )

    def test_a_verifiable_example_is_run_headless_rather_than_only_built(self):
        # The point of I2: an example that needs a person to look at it gets the
        # looking scripted, instead of being compiled and shrugged at.
        example = sorted(test_wrapper.VERIFIABLE_EXAMPLES)[0]
        name, command = test_wrapper.example_phase("jidousha-platform", example)
        self.assertEqual(name, f"example-verify:{example}")
        self.assertIn("tools/verify", command)
        self.assertIn(example, command)

    def test_every_verifiable_example_is_a_windowed_one(self):
        # A headless example already asserts in its normal mode; giving it a
        # second mode would be a second way to do one thing, and this phase
        # would replace the run that was already checking it.
        self.assertTrue(
            test_wrapper.VERIFIABLE_EXAMPLES <= test_wrapper.WINDOWED_EXAMPLES,
            "a verifiable example that is not windowed would stop being run normally",
        )


class VerifyToolTest(unittest.TestCase):
    OUTPUT = (
        "verified prototype_kit over 130 ticks\n"
        "  paddle: 0.00 -> 7.00, clamped\n"
        "  frames: 130\n"
        "clear #121721ff\n"
        "batch 0: texture 0 (21 quads)\n"
        "  quad (0.0, 0.0) (1.0, 1.0) tint #ffffffff\n"
    )

    def test_the_verdict_and_its_summary_are_read_out_of_the_output(self):
        verdict, summary = verify.parse_verdict(self.OUTPUT)
        self.assertEqual(verdict, "verified prototype_kit over 130 ticks")
        self.assertEqual(summary, ["paddle: 0.00 -> 7.00, clamped", "frames: 130"])

    def test_the_draw_transcript_is_not_mistaken_for_the_summary(self):
        # The transcript's own quad lines are indented too. Stopping at the
        # first unindented line is what keeps them out — without it the summary
        # would be the whole frame.
        _, summary = verify.parse_verdict(self.OUTPUT)
        self.assertNotIn("quad (0.0, 0.0) (1.0, 1.0) tint #ffffffff", summary)

    def test_output_with_no_verdict_line_reads_as_unverified(self):
        # An example that ignored `--verify` and ran normally exits 0 having
        # verified nothing. Calling that a pass is the one failure mode this
        # whole script has to avoid.
        self.assertEqual(verify.parse_verdict("hello\nworld\n"), (None, []))

    def test_a_verdict_with_nothing_under_it_still_reads_as_a_verdict(self):
        self.assertEqual(
            verify.parse_verdict("verified thing over 3 ticks\n"),
            ("verified thing over 3 ticks", []),
        )

    def test_an_example_is_looked_up_by_name_across_the_workspace(self):
        examples = [("jidousha-core", "homing"), ("jidousha-platform", "prototype_kit")]
        self.assertEqual(verify.find_example(examples, "prototype_kit"), "jidousha-platform")

    def test_an_unknown_example_is_not_resolved_to_some_other_package(self):
        examples = [("jidousha-core", "homing")]
        self.assertIsNone(verify.find_example(examples, "nope"))

    def test_a_clean_run_with_a_verdict_is_a_pass(self):
        self.assertEqual(verify.verdict_status("ok", "verified thing over 3 ticks"), "pass")

    def test_a_clean_run_with_no_verdict_is_not_a_pass(self):
        # The one failure mode this script exists to avoid: an example that
        # ignores `--verify` runs normally, exits 0, and asserts nothing.
        self.assertEqual(verify.verdict_status("ok", None), "unverified")
        self.assertEqual(verify.EXIT_CODES["unverified"], 2)

    def test_a_nonzero_exit_is_a_failure_whatever_it_printed(self):
        # An example that failed an assertion after printing its verdict would
        # otherwise be read as a pass.
        self.assertEqual(verify.verdict_status("failed", "verified thing"), "fail")
        self.assertEqual(verify.EXIT_CODES["fail"], 1)

    def test_a_timeout_and_a_launch_failure_keep_their_own_names(self):
        # Both are tooling faults, and neither should be reported as the
        # example's assertions failing.
        self.assertEqual(verify.verdict_status("timeout", None), "timeout")
        self.assertEqual(verify.verdict_status("error", None), "error")
        self.assertEqual(verify.EXIT_CODES["timeout"], 2)
        self.assertEqual(verify.EXIT_CODES["error"], 2)

    def test_a_captured_frame_is_lifted_out_of_the_summary(self):
        # So an agent looking for the picture does not have to parse English.
        text = (
            "verified prototype_kit over 130 ticks\n"
            "  frames: 130\n"
            "  capture: 480x270 written to /repo/target/verify/prototype_kit.png\n"
        )
        self.assertEqual(
            verify.parse_artifact(text), "/repo/target/verify/prototype_kit.png"
        )

    def test_a_run_that_captured_nothing_reports_no_artifact(self):
        # A machine with no GPU. Not a failure, and not a path either.
        text = "verified thing over 3 ticks\n  capture: skipped, no GPU on this machine\n"
        self.assertIsNone(verify.parse_artifact(text))

    def test_a_line_that_merely_mentions_writing_is_not_an_artifact(self):
        # The marker is anchored to the `capture:` line, so a transcript that
        # happens to contain the words does not become the report's artifact.
        text = "verified thing\n  note: nothing was written to disk this run\n"
        self.assertIsNone(verify.parse_artifact(text))

    def test_the_report_does_not_go_where_the_test_wrapper_writes_its_own(self):
        # Two tools writing one ground-truth file is how ground truth stops
        # being true (tooling.md §3).
        self.assertNotEqual(verify.REPORT_DIR / "prototype_kit.json", test_wrapper.REPORT_PATH)
        self.assertEqual(verify.REPORT_DIR, test_wrapper.REPORT_PATH.parent)


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

    def test_a_canvas_cleared_to_the_pages_own_color_still_reads_as_drawn(self):
        # The blind spot the first rule alone has, found by `input_echo`: it
        # clears to a color a shade off the page's, so almost nothing "differs
        # from the background" — but it drew a readout, and a blank canvas would
        # not have. Two shades of near-black plus one bright row is what that
        # looks like at this size.
        rows = [[(15, 18, 26)] * 8 for _ in range(8)]
        rows[6] = [(200, 220, 255)] * 8
        drawn, detail = serve_web.canvas_is_drawn(png_bytes(8, 8, rows))
        self.assertTrue(drawn, detail)

    def test_a_canvas_of_one_flat_color_near_the_background_reads_as_blank(self):
        # And the other side of it: the second rule must not accept a page that
        # merely cleared to something close to the background and drew nothing,
        # or it would pass the exact failure this whole check exists for.
        rows = [[(15, 18, 26)] * 8 for _ in range(8)]
        drawn, detail = serve_web.canvas_is_drawn(png_bytes(8, 8, rows))
        self.assertFalse(drawn, detail)

    def test_the_wasm_bindgen_version_is_read_from_the_lockfile(self):
        # The CLI and the crate generate two halves of one interface, so this
        # is what stops a skew from becoming a runtime mystery.
        version = serve_web.locked_wasm_bindgen_version()
        self.assertIsNotNone(version, "Cargo.lock should pin wasm-bindgen")
        self.assertRegex(version, r"^\d+\.\d+\.\d+$")
