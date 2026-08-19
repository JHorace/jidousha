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
import re
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
gen_api_doc = load_tool("gen-api-doc")
api_coverage = load_tool("check-api-coverage")


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


FACADE = """
// --- App and lifecycle ---------------------------------------------------
pub use jidousha_core::{App, Draw, headless};

// --- Render ----------------------------------------------------------------
pub use jidousha_render_core::{Camera, Sprite};
pub use jidousha_core::math;

pub mod prelude {
    pub use crate::{App, Camera, Draw, Sprite, headless};
}

pub mod testing {
    pub use jidousha_input::{InputScript};
}
"""


class GenApiDocTest(unittest.TestCase):
    def test_the_reference_is_grouped_by_the_facades_own_banners(self):
        # The grouping is the facade's, not a list kept in the generator: move
        # an item between sections there and the documentation follows.
        groups = gen_api_doc.facade_exports(FACADE)
        self.assertEqual([title for title, _ in groups], ["App and lifecycle", "Render"])
        self.assertEqual(groups[0][1], ["App", "Draw", "headless"])

    def test_a_single_item_re_export_is_found_too(self):
        # `pub use jidousha_core::math;` has no braces and would otherwise be
        # silently absent from the documentation.
        groups = dict(gen_api_doc.facade_exports(FACADE))
        self.assertIn("math", groups["Render"])

    def test_the_prelude_is_not_counted_as_a_second_surface(self):
        # It re-exports the same names; listing them twice would say nothing.
        names = [name for _, group in gen_api_doc.facade_exports(FACADE) for name in group]
        self.assertEqual(len(names), len(set(names)))

    def test_the_testing_module_is_read_separately(self):
        self.assertEqual(gen_api_doc.testing_exports(FACADE), ["InputScript"])

    def test_a_summary_is_the_whole_first_sentence(self):
        # Doc comments wrap at eighty columns, so taking the first *line* gives
        # a reference full of summaries that stop mid-clause.
        block = ["What a Draw system is called with: the world to read, and the", "sink to draw into.", "", "More prose."]
        self.assertEqual(
            gen_api_doc.first_sentence(block),
            "What a Draw system is called with: the world to read, and the sink to draw into",
        )

    def test_a_summary_stops_at_the_end_of_the_first_sentence(self):
        block = ["A duration, in seconds.", "Not milliseconds, and not ticks."]
        self.assertEqual(gen_api_doc.first_sentence(block), "A duration, in seconds")

    def test_intra_doc_link_brackets_are_stripped(self):
        # They mean nothing in a markdown file an agent reads.
        self.assertEqual(gen_api_doc.first_sentence(["Yields `()`, like [`With`]."]),
                         "Yields `()`, like `With`")

    def test_a_summary_is_taken_from_the_doc_comment_above_the_definition(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "thing.rs"
            source.write_text(
                "/// A duration, in seconds.\n"
                "#[derive(Clone)]\n"
                "pub struct Seconds(pub f32);\n"
            )
            self.assertEqual(
                gen_api_doc.doc_summaries([source]), {"Seconds": "A duration, in seconds"}
            )

    def test_an_undocumented_item_simply_has_no_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "thing.rs"
            source.write_text("pub struct Bare;\n")
            self.assertEqual(gen_api_doc.doc_summaries([source]), {})

    def test_implementation_vocabulary_is_refused(self):
        self.assertIn("jidousha_core", gen_api_doc.forbidden_words("see jidousha_core::math"))
        self.assertIn("archetype", gen_api_doc.forbidden_words("stored in an Archetype"))
        self.assertEqual(gen_api_doc.forbidden_words("a plain sentence"), [])

    def test_only_the_document_given_the_exception_may_use_it(self):
        # The exception used to be a *section* cut out of the check, which meant
        # everything else forbidden could sit inside that section unnoticed. It
        # is a per-document parameter now (ADR-0025): the testing document may
        # name a renderer, and nothing else is excused anywhere.
        text = "the capture goes through wgpu"
        self.assertIn("wgpu", gen_api_doc.forbidden_words(text))
        self.assertEqual(gen_api_doc.forbidden_words(text, gen_api_doc.TESTING_VOCABULARY), [])
        # The exception is three words wide and covers nothing else, so a
        # document holding the exception is still held to all the rest.
        for refused in ("see jidousha_render_core", "stored in an Archetype", "see ADR-0010"):
            self.assertNotEqual(
                gen_api_doc.forbidden_words(refused, gen_api_doc.TESTING_VOCABULARY),
                [],
                refused,
            )

    def test_a_citation_of_a_maintainers_document_is_refused(self):
        # E0 run 1 read `**message** — The failure in the engine's message
        # format (core.md §9)` and could not follow the pointer: the prompt
        # forbids `docs/internal/` outright (F-005). The gate had a list of
        # crate names and seam types and no notion of a document path, so it
        # read as covering a class it covered half of.
        self.assertIn("core.md", gen_api_doc.forbidden_words("the format (core.md §9)"))
        self.assertIn("ADR-", gen_api_doc.forbidden_words("clockwise on screen, see ADR-0010"))
        self.assertIn("docs/internal", gen_api_doc.forbidden_words("see docs/internal/renderer.md"))

    def test_a_parenthetical_citation_is_stripped_from_the_game_facing_text(self):
        # A doc comment serves two readers: rustdoc, where `(ADR-0010)` is the
        # point, and this document, whose reader may not open `docs/adr/` at
        # all. Stripping on the way out keeps the citation for the reader it
        # helps rather than deleting it from the source.
        self.assertEqual(
            gen_api_doc.scrub_internal_references("Rotation, clockwise on screen (ADR-0010)"),
            "Rotation, clockwise on screen",
        )
        self.assertEqual(
            gen_api_doc.scrub_internal_references("The engine's message format (core.md §9)"),
            "The engine's message format",
        )
        self.assertEqual(
            gen_api_doc.scrub_internal_references("The whole snapshot, for the recorder (I2)"),
            "The whole snapshot, for the recorder",
        )
        # Ordinary parentheses are not citations and must survive.
        self.assertEqual(
            gen_api_doc.scrub_internal_references("Width and height (in world units)"),
            "Width and height (in world units)",
        )

    def test_the_game_document_names_no_renderer_and_has_no_exemption(self):
        # Stronger than it used to be, and worth stating: the whole
        # `jidousha::testing` block used to be cut out of this check, so an
        # internal crate name or a pointer into `docs/internal/` could sit
        # inside it unnoticed. The block is a separate document now, and the
        # game document is checked entire, with nothing allowed through
        # (ADR-0025).
        root = Path(__file__).resolve().parents[2]
        text = (root / "docs/api/jidousha-api.md").read_text(encoding="utf-8")
        self.assertEqual(gen_api_doc.forbidden_words(text), [])

    def test_the_testing_document_may_name_a_renderer_and_nothing_else(self):
        # A picture has to be drawn by something, so the capture recipe cannot
        # be written without naming the renderer it is written against. That is
        # the whole exception, and this pins its size: three words. Everything
        # else — internal crates, archetype storage, pointers into
        # `docs/internal/` — is as forbidden here as in the game document.
        root = Path(__file__).resolve().parents[2]
        text = (root / "docs/api/jidousha-testing.md").read_text(encoding="utf-8")
        self.assertEqual(
            gen_api_doc.forbidden_words(text, gen_api_doc.TESTING_VOCABULARY), []
        )
        # And the exception is really being used, so a later reader does not
        # conclude it could simply be deleted.
        self.assertEqual(
            gen_api_doc.forbidden_words(text), list(gen_api_doc.TESTING_VOCABULARY)
        )

    def test_an_entry_example_is_the_doctest_with_its_setup_removed(self):
        # public-api.md §4's entry spec is "signature, one-liner, tiny example",
        # and the example third went unbuilt until the budget had room. These
        # are the crate's own doctests, so the example in the document is code
        # CI compiles.
        block = [
            "Turn `vector` by `angle`.",
            "",
            "```",
            "# use jidousha_core::math::{Radians, Vec2, rotate};",
            "let turned = rotate(Vec2::new(1.0, 0.0), Radians::from_degrees(90.0));",
            "```",
        ]
        self.assertEqual(
            gen_api_doc.doc_example(block),
            ["let turned = rotate(Vec2::new(1.0, 0.0), Radians::from_degrees(90.0));"],
        )

    def test_an_entry_example_never_names_an_internal_crate(self):
        # Three ways a doctest names its own crate, all of which would put the
        # whole FORBIDDEN list into the document by the back door: a hidden
        # setup line, a visible import, and a path written out mid-expression.
        # The document's own second sentence is "everything here is reachable
        # from one import", so dropping them makes the example more correct for
        # the reader it is shown to, not less.
        block = [
            "```",
            "# use jidousha_core::World;",
            "use jidousha_assets::Assets;",
            "let mut assets = Assets::new(jidousha_platform::asset_source(\"assets\"));",
            "```",
        ]
        rendered = gen_api_doc.doc_example(block)
        self.assertEqual(rendered, ['let mut assets = Assets::new(asset_source("assets"));'])
        self.assertEqual(gen_api_doc.forbidden_words("\n".join(rendered)), [])

    def test_an_entry_example_cites_no_document_its_reader_may_not_open(self):
        block = ["```", "// The frame is owned (ADR-0023).", "let first = frames[0];", "```"]
        self.assertEqual(
            gen_api_doc.doc_example(block),
            ["// The frame is owned.", "let first = frames[0];"],
        )

    def test_a_doctest_that_is_only_setup_shows_no_example(self):
        # Rendering an empty code fence would be worse than rendering nothing:
        # it reads as "there is an example and it is blank".
        self.assertEqual(gen_api_doc.doc_example(["```", "# use jidousha_core::World;", "```"]), [])
        self.assertEqual(gen_api_doc.doc_example(["No code here at all."]), [])

    def test_a_literal_hash_survives_rustdocs_escape(self):
        self.assertEqual(gen_api_doc.doc_example(["```", "## not setup", "```"]), ["# not setup"])

    def test_the_committed_documents_carry_worked_examples(self):
        # A floor, like the anti-shrink one: the extractor silently returning
        # nothing would take the whole feature out and change no other check.
        #
        # Counting code blocks against entries is NOT enough and was tried: the
        # Quickstart and Concepts carry blocks of their own, so "more blocks
        # than entries" stays true with every example removed. Checked by
        # mutation — that version passed with the extractor stubbed to return
        # nothing, which is the whole failure it was supposed to catch.
        #
        # What is specific to an entry carrying an example is the *shape*: a
        # declaration block closed and a second block opened immediately after.
        root = Path(__file__).resolve().parents[2]
        for name, floor in (("jidousha-api.md", 20), ("jidousha-testing.md", 5)):
            with self.subTest(document=name):
                text = (root / "docs/api" / name).read_text(encoding="utf-8")
                self.assertGreater(text.count("```\n\n```rust"), floor)

    def test_the_extractor_finds_examples_in_the_real_sources(self):
        # The other end of the same guard. The check above reads the committed
        # documents; this one reads the crates, so a doc comment convention that
        # drifts out from under the extractor fails here rather than quietly
        # emptying the reference.
        root = Path(__file__).resolve().parents[2]
        items = gen_api_doc.scan_sources(gen_api_doc.crate_sources(root))
        self.assertGreater(len([item for item in items.values() if item.example]), 20)

    def test_a_pointer_at_a_worked_example_that_is_not_there_is_refused(self):
        root = Path(__file__).resolve().parents[2]
        self.assertEqual(
            gen_api_doc.dangling_examples("the whole loop is `examples/no_such.rs`", root),
            ["examples/no_such.rs"],
        )
        self.assertEqual(
            gen_api_doc.dangling_examples("see `examples/homing.rs` and `examples/`.", root),
            [],
        )

    def test_a_pointer_at_the_example_a_run_writes_is_refused_while_it_still_exists(self):
        # The half of the check that "does the file exist" cannot do, and the
        # reason the check exists at all. `e0-prompt.md`'s before-the-run step 2
        # deletes `crates/jidousha/examples/pong/`, so a pointer at it is
        # correct on the day it is written and dangling on the day a run would
        # follow it. A stat-only check passes on the commit that introduces the
        # bug and fails nowhere afterwards — which is exactly what happened.
        root = Path(__file__).resolve().parents[2]
        self.assertTrue((root / "crates/jidousha/examples/pong").exists(), "on disk today")
        self.assertEqual(
            gen_api_doc.dangling_examples("written down in `examples/pong/capture.rs`", root),
            ["examples/pong/capture.rs (deleted before the next run)"],
        )

    def test_the_committed_documents_point_only_at_examples_that_outlive_a_run(self):
        root = Path(__file__).resolve().parents[2]
        for name in ("jidousha-api.md", "jidousha-testing.md"):
            with self.subTest(document=name):
                text = (root / "docs/api" / name).read_text(encoding="utf-8")
                self.assertEqual(gen_api_doc.dangling_examples(text, root), [])

    def test_each_document_is_counted_against_its_own_budget(self):
        # The budget is the point (public-api.md §4): everything in a document
        # has to be relevant to what its reader is doing. Two readers, two
        # numbers — a single number over both would let the testing half eat
        # the game half's room again, which is what ADR-0025 is about.
        for document in gen_api_doc.documents(Path(__file__).resolve().parents[2]):
            with self.subTest(document=document.path.name):
                self.assertLess(gen_api_doc.token_estimate(document.text), document.budget)

    def test_the_committed_documents_are_what_the_facade_generates(self):
        # The same thing CI checks, so a stale document fails here first.
        self.assertEqual(gen_api_doc.main(["gen-api-doc", "--check"]), 0)

    def test_a_stale_document_fails_the_check(self):
        # The other half, and the one that matters: without it the staleness
        # branch could be deleted and every test above would still pass. A
        # document that silently stops matching the code is worse than none,
        # because an agent believes it.
        #
        # Run over *each* document: one check that only ever looked at the game
        # document would pass while the testing document rotted, which is the
        # failure a split invites and the reason the check is written once and
        # applied to a list.
        for attribute in ("GAME_OUTPUT", "TESTING_OUTPUT"):
            with self.subTest(document=attribute):
                original = getattr(gen_api_doc, attribute)
                with tempfile.TemporaryDirectory() as directory:
                    stale = Path(directory) / original.name
                    stale.write_text("# not what the facade generates\n")
                    setattr(gen_api_doc, attribute, stale)
                    try:
                        out, err = io.StringIO(), io.StringIO()
                        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                            code = gen_api_doc.main(["gen-api-doc", "--check"])
                        self.assertEqual(code, 1)
                        self.assertIn("stale", err.getvalue())
                    finally:
                        setattr(gen_api_doc, attribute, original)

    def test_a_document_over_budget_fails(self):
        # A budget nothing enforces is a number in a comment. Both, for the
        # same reason the staleness check runs over both.
        for attribute in ("GAME_BUDGET", "TESTING_BUDGET"):
            with self.subTest(budget=attribute):
                original = getattr(gen_api_doc, attribute)
                setattr(gen_api_doc, attribute, 1)
                try:
                    out, err = io.StringIO(), io.StringIO()
                    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                        code = gen_api_doc.main(["gen-api-doc", "--check"])
                    self.assertEqual(code, 1)
                    self.assertIn("over the 1 budget", err.getvalue())
                finally:
                    setattr(gen_api_doc, attribute, original)

    def test_the_game_document_points_at_the_testing_document(self):
        # The one cost a split surface has to pay: an agent that does not know
        # the second file exists will not find it. Both places a reader would
        # look — the Reference group that used to hold the testing signatures,
        # and the section that used to hold the prose — name it.
        root = Path(__file__).resolve().parents[2]
        text = (root / "docs/api/jidousha-api.md").read_text(encoding="utf-8")
        self.assertEqual(text.count("docs/api/jidousha-testing.md"), 3)
        self.assertIn("### Testing (`jidousha::testing`)", text)
        self.assertIn("## Testing your game", text)


def scan_snippet(*texts):
    """Scan fragments of Rust the way the generator scans crate sources.

    Several fragments stand for several files, named in the order given — which
    is how a test says "this `impl` block is in a file that sorts before the one
    declaring its type".
    """
    files = [(text.splitlines(), f"snippet{number}.rs") for number, text in enumerate(texts)]
    items = {}
    for phase in (gen_api_doc.DECLARATIONS, gen_api_doc.ATTACHMENTS):
        for lines, source in files:
            gen_api_doc.scan_file(lines, source, items, phase)
    for item in items.values():
        gen_api_doc.order_members(item)
    return items


class ApiExtractionTest(unittest.TestCase):
    """The declaration extractor, on synthetic sources.

    E0 run 1 could not make a single call from the API document, because the
    reference carried names and no signatures (e0-findings.md F-001). These
    cover the shapes that got it wrong on the way to fixing that.
    """

    def test_a_structs_public_fields_are_listed_with_their_types(self):
        items = scan_snippet(
            "/// What the frame is looking at.\n"
            "pub struct Camera {\n"
            "    /// The world position at the center of the screen.\n"
            "    pub center: Vec2,\n"
            "    /// How many world units the screen spans vertically.\n"
            "    pub height: f32,\n"
            "}\n"
        )
        self.assertEqual(
            items["Camera"].fields,
            [
                ("pub center: Vec2", "The world position at the center of the screen"),
                ("pub height: f32", "How many world units the screen spans vertically"),
            ],
        )

    def test_a_private_field_is_not_listed(self):
        # `Entity` has exactly two fields and both are private. A document that
        # showed them would teach a game to reach for what it cannot have.
        items = scan_snippet(
            "pub struct Entity {\n    index: u32,\n    generation: NonZeroU32,\n}\n"
        )
        self.assertEqual(items["Entity"].fields, [])
        # ...but the block was still read, which is what tells a fully private
        # type apart from one the scanner failed to enter.
        self.assertEqual(items["Entity"].body_lines, 2)

    def test_a_multi_line_signature_is_joined_onto_one_line(self):
        # rustfmt wraps long argument lists and closes at the item's own
        # indentation, which is the terminator `read_signature` relies on.
        items = scan_snippet(
            "pub struct World {\n    slots: Vec<u32>,\n}\n"
            "\n"
            "impl World {\n"
            "    /// Give `entity` a `T`.\n"
            "    pub fn try_insert<T: Component>(\n"
            "        &mut self,\n"
            "        entity: Entity,\n"
            "        value: T,\n"
            "    ) -> Result<(), EntityDeadError> {\n"
            "        todo!()\n"
            "    }\n"
            "}\n"
        )
        self.assertEqual(
            items["World"].members,
            [
                (
                    "pub fn try_insert<T: Component>(&mut self, entity: Entity, "
                    "value: T) -> Result<(), EntityDeadError>;",
                    "Give `entity` a `T`",
                )
            ],
        )

    def test_a_where_clause_is_carried_onto_the_signature_line(self):
        # `App::add_system`'s bound is the useful half of that signature.
        items = scan_snippet(
            "pub struct App {\n    simulation: Simulation,\n}\n"
            "\n"
            "impl App {\n"
            "    /// Append a system to a phase.\n"
            "    pub fn add_system<P, F>(&mut self, phase: P, system: F)\n"
            "    where\n"
            "        P: Phase,\n"
            "        F: IntoSystem<P>,\n"
            "    {\n"
            "        todo!()\n"
            "    }\n"
            "}\n"
        )
        self.assertIn("where P: Phase, F: IntoSystem<P>", items["App"].members[0][0])

    def test_a_generic_impl_block_is_attributed_to_its_type(self):
        # `impl<'w, Q: Query<'w>> QueryIterMut<'w, Q>` is why the generic list
        # is skipped by counting angle brackets: the first `>` closes an inner
        # parameter, not the list.
        items = scan_snippet(
            "pub struct DrawCtx<'w> {\n    /// The world.\n    pub world: WorldView<'w>,\n}\n"
            "\n"
            "impl<'w> DrawCtx<'w> {\n"
            "    /// Draw one quad.\n"
            "    pub fn submit(&mut self, quad: Quad) {\n        todo!()\n    }\n"
            "}\n"
        )
        self.assertEqual(
            items["DrawCtx"].members, [("pub fn submit(&mut self, quad: Quad);", "Draw one quad")]
        )

    def test_a_trait_impl_does_not_add_its_methods_to_the_type(self):
        # `ctx.rect(..)` resolves through `Submit`, and the trait's own
        # declaration is where that signature belongs. Listing it as inherent
        # would say, wrongly, that it resolves without the trait in scope.
        items = scan_snippet(
            "pub struct DrawCtx<'w> {\n    /// The world.\n    pub world: WorldView<'w>,\n}\n"
            "\n"
            "impl Submit for DrawCtx<'_> {\n"
            "    fn rect(&mut self, rect: Rect, color: Color, depth: Depth) {\n"
            "        todo!()\n"
            "    }\n"
            "}\n"
        )
        self.assertEqual(items["DrawCtx"].members, [])
        self.assertEqual(items["DrawCtx"].traits, ["Submit"])

    def test_a_brace_inside_a_string_literal_does_not_close_the_block(self):
        # `write!(formatter, "TextureId({bits})")` is the case a depth counter
        # gets wrong, closing two blocks early and losing whatever is defined
        # next. Blocks close on indentation for exactly this reason.
        items = scan_snippet(
            "pub struct TextureId(u64);\n"
            "\n"
            "impl fmt::Debug for TextureId {\n"
            "    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {\n"
            '        write!(formatter, "TextureId({bits})")\n'
            "    }\n"
            "}\n"
            "\n"
            "/// A duration, in seconds.\n"
            "pub struct Seconds(pub f32);\n"
        )
        self.assertEqual(items["Seconds"].summary, "A duration, in seconds")

    def test_a_const_fn_is_not_read_as_an_associated_const(self):
        # `pub const fn layer(..)` opens a body like any other function; only a
        # real associated const runs to its semicolon. Reading one as the other
        # swallowed twenty-five lines and lost the next type entirely.
        items = scan_snippet(
            "pub struct Depth {\n    /// The band.\n    pub layer: i16,\n}\n"
            "\n"
            "impl Depth {\n"
            "    /// The front of `layer`'s band.\n"
            "    pub const fn layer(layer: i16) -> Self {\n"
            "        Self { layer, z: 0.0 }\n"
            "    }\n"
            "}\n"
            "\n"
            "/// Which texture a quad samples.\n"
            "pub struct TextureId(u64);\n"
        )
        self.assertEqual(items["TextureId"].summary, "Which texture a quad samples")

    def test_an_associated_const_keeps_its_value(self):
        items = scan_snippet(
            "pub struct Rect {\n    /// Top-left.\n    pub min: Vec2,\n}\n"
            "\n"
            "impl Rect {\n"
            "    /// The whole of something.\n"
            "    pub const UNIT: Rect = Rect {\n"
            "        min: Vec2::ZERO,\n"
            "        max: Vec2::ONE,\n"
            "    };\n"
            "}\n"
        )
        self.assertEqual(
            items["Rect"].members[0][0],
            "pub const UNIT: Rect = Rect { min: Vec2::ZERO, max: Vec2::ONE };",
        )

    def test_a_macro_templated_const_initialiser_is_dropped(self):
        # `&[$(Key::$name),*]` is not something a game can read or write.
        items = scan_snippet(
            "pub struct Key;\n"
            "\n"
            "impl Key {\n"
            "    /// Every key, in declaration order.\n"
            "    pub const ALL: &'static [Key] = &[$(Key::$name),*];\n"
            "}\n"
        )
        self.assertEqual(items["Key"].members[0][0], "pub const ALL: &'static [Key];")

    def test_an_enum_variant_with_a_struct_body_keeps_its_fields(self):
        # `RunError::NoDisplay { detail: String }`, and the `},` that closes it
        # — a closer whose trailing comma an exact-match test misses.
        items = scan_snippet(
            "pub enum RunError {\n"
            "    /// There is no display to open a window on.\n"
            "    NoDisplay {\n"
            "        /// What the platform said.\n"
            "        detail: String,\n"
            "    },\n"
            "    /// The event loop stopped with an error.\n"
            "    EventLoop {\n"
            "        /// What the platform said.\n"
            "        detail: String,\n"
            "    },\n"
            "}\n"
        )
        self.assertEqual(
            [text for text, _ in items["RunError"].variants],
            ["NoDisplay { detail: String }", "EventLoop { detail: String }"],
        )

    def test_the_default_value_is_read_from_the_default_impl(self):
        # `GameConfig::default()` is where a game learns the tick rate is 1/60,
        # and that number appears nowhere else a game may look (F-004).
        items = scan_snippet(
            "pub struct GameConfig {\n    /// The seed.\n    pub seed: u64,\n}\n"
            "\n"
            "impl Default for GameConfig {\n"
            "    fn default() -> Self {\n"
            "        Self {\n"
            "            seed: 0,\n"
            "            fixed_dt: Seconds(1.0 / 60.0),\n"
            "        }\n"
            "    }\n"
            "}\n"
        )
        self.assertEqual(
            items["GameConfig"].default_value, "Self { seed: 0, fixed_dt: Seconds(1.0 / 60.0) }"
        )

    def test_an_empty_trait_body_is_recognised_rather_than_read_past(self):
        # `pub trait Resource: 'static + Send + Sync {}` ends the declaration as
        # surely as a `{` does; reading past it swallowed the next definition.
        items = scan_snippet(
            "/// Marks a type as storable as a world resource.\n"
            "pub trait Resource: 'static + Send + Sync {}\n"
            "\n"
            "/// Every resource in the world.\n"
            "pub struct Resources {\n    slots: Vec<u32>,\n}\n"
        )
        self.assertEqual(items["Resource"].decl, "pub trait Resource: 'static + Send + Sync {}")
        self.assertTrue(items["Resource"].empty_body)
        self.assertEqual(items["Resources"].summary, "Every resource in the world")

    def test_an_internal_crate_path_is_dropped_from_a_signature(self):
        # `asset_source` returns `impl jidousha_assets::ByteSource`. The type is
        # the same type; the path is the routing a facade exists to hide, and
        # naming it would send a game author looking for a crate it must not
        # depend on.
        items = scan_snippet(
            "/// The asset source this platform reads with.\n"
            "pub fn asset_source(root: &str) -> impl jidousha_assets::ByteSource {\n"
            "    todo!()\n"
            "}\n"
        )
        self.assertEqual(
            items["asset_source"].decl, "pub fn asset_source(root: &str) -> impl ByteSource"
        )

    def test_a_macro_declared_enum_is_resolved_from_its_invocation(self):
        # `pub enum Key {` inside `macro_rules! keys` has a body of `$( $name, )*`.
        # Scanning it finds no variants, and reporting that as fact is what E0
        # read as "this keyboard has no keys" (F-002).
        items = scan_snippet(
            "macro_rules! keys {\n"
            "    ($($name:ident = $code:expr),* $(,)?) => {\n"
            "        /// A physical key.\n"
            "        pub enum Key {\n"
            "            $(\n"
            "                $name,\n"
            "            )*\n"
            "        }\n"
            "    };\n"
            "}\n"
            "\n"
            "keys! {\n"
            "    // Letters.\n"
            "    A = 1, B = 2,\n"
            "\n"
            "    ArrowUp = 40, Escape = 52,\n"
            "}\n"
        )
        self.assertEqual(items["Key"].variants_from, "macro:keys")
        self.assertEqual(
            [text for text, _ in items["Key"].variants],
            ["// Letters.", "A, B,", "", "ArrowUp, Escape,"],
        )

    def test_an_impl_block_in_a_file_that_sorts_first_still_contributes(self):
        # E0 run 2's largest gap, and it was not a missing doc comment: the
        # scanner read `crates/jidousha-core/src/resource.rs` before
        # `world.rs`, looked `World` up, found nothing, and dropped the whole
        # block. Five resource methods went missing from a reference whose own
        # Quickstart calls three of them (e0-findings.md F-016).
        items = scan_snippet(
            "impl World {\n"
            "    /// Store a resource, replacing any of the same type.\n"
            "    pub fn insert_resource<T: Resource>(&mut self, value: T) {\n"
            "        self.resources.insert(value);\n"
            "    }\n"
            "}\n",
            "/// Everything the simulation can see.\n"
            "pub struct World {\n    slots: Vec<u32>,\n}\n"
            "\n"
            "impl World {\n"
            "    /// Create an empty world.\n"
            "    pub fn new() -> Self {\n"
            "        Self { slots: Vec::new() }\n"
            "    }\n"
            "}\n",
        )
        self.assertEqual(
            items["World"].members,
            [
                ("pub fn new() -> Self;", "Create an empty world"),
                (
                    "pub fn insert_resource<T: Resource>(&mut self, value: T);",
                    "Store a resource, replacing any of the same type",
                ),
            ],
            "the type's own block first, then the block extending it elsewhere",
        )

    def test_a_trait_impl_in_a_file_that_sorts_first_still_badges_its_type(self):
        # Same bug, the other half: `impl Default for X` read before `X` was
        # declared lost both the badge and the default value, which is how a
        # reader learns the tick rate is 1/60.
        items = scan_snippet(
            "impl Default for GameConfig {\n"
            "    fn default() -> Self {\n"
            "        Self { fixed_dt: Seconds(1.0 / 60.0) }\n"
            "    }\n"
            "}\n",
            "/// How a run is configured.\npub struct GameConfig {\n    pub seed: u64,\n}\n",
        )
        self.assertEqual(items["GameConfig"].traits, ["Default"])
        self.assertEqual(items["GameConfig"].default_value, "Self { fixed_dt: Seconds(1.0 / 60.0) }")

    def test_a_field_is_not_recorded_twice_by_the_second_pass(self):
        # The declaration pass walks every file and so does the attachment
        # pass. Both open the same blocks; only one may write.
        items = scan_snippet(
            "/// What the frame is looking at.\n"
            "pub struct Camera {\n    pub center: Vec2,\n    pub height: f32,\n}\n"
        )
        self.assertEqual(len(items["Camera"].fields), 2)
        self.assertEqual(items["Camera"].body_lines, 2)


class ApiCompletenessTest(unittest.TestCase):
    """The gate that makes a thin reference loud.

    This document's failure mode is not being wrong, it is being thin — and a
    thin entry looks exactly like a complete one to the agent reading it. Every
    test here is the negative half of a rule, which is the half that keeps the
    rule alive.
    """

    def groups(self, *names):
        return [("Group", list(names))]

    def test_an_exported_item_with_no_definition_anywhere_fails(self):
        failures = gen_api_doc.completeness_failures(self.groups("Ghost"), [], {})
        self.assertEqual(len(failures), 1)
        self.assertIn("`Ghost`", failures[0])

    def test_an_enum_that_yields_no_variants_fails(self):
        items = scan_snippet(
            "macro_rules! keys {\n"
            "    ($($name:ident = $code:expr),*) => {\n"
            "        /// A physical key.\n"
            "        pub enum Key {\n"
            "            $($name,)*\n"
            "        }\n"
            "    };\n"
            "}\n"
        )
        failures = gen_api_doc.completeness_failures(self.groups("Key"), [], items)
        self.assertTrue(any("no variants" in failure for failure in failures))

    def test_a_registered_macro_whose_invocation_is_missing_fails(self):
        # The macro was renamed or moved and the generator silently stopped
        # understanding it. Ninety-six variants would go missing quietly.
        items = scan_snippet("/// A physical key.\npub enum Key {\n    Space,\n}\n")
        failures = gen_api_doc.completeness_failures(self.groups("Key"), [], items)
        self.assertTrue(any("keys!" in failure for failure in failures))

    def test_a_type_whose_members_are_all_private_is_not_a_failure(self):
        # `Entity` is a legitimately opaque handle: private fields, `pub(crate)`
        # methods, and nothing for this document to show. The rule has to tell
        # that apart from a block that was never entered.
        items = scan_snippet(
            "/// A handle to a thing in the world.\n"
            "pub struct Entity {\n    index: u32,\n    generation: NonZeroU32,\n}\n"
        )
        self.assertEqual(gen_api_doc.completeness_failures(self.groups("Entity"), [], items), [])

    def test_a_marker_trait_is_not_a_failure(self):
        items = scan_snippet(
            "/// Marks a plain-data type as storable on an entity.\n"
            "pub trait Component: 'static + Send + Sync {}\n"
        )
        self.assertEqual(gen_api_doc.completeness_failures(self.groups("Component"), [], items), [])

    def test_the_real_surface_is_complete(self):
        # The same gate CI runs, against the real crates: every item the facade
        # exports yields at least a declaration.
        root = Path(__file__).resolve().parents[2]
        facade = (root / "crates/jidousha/src/lib.rs").read_text(encoding="utf-8")
        items = gen_api_doc.scan_sources(gen_api_doc.crate_sources(root))
        failures = gen_api_doc.completeness_failures(
            gen_api_doc.facade_exports(facade), gen_api_doc.testing_exports(facade), items
        )
        self.assertEqual(failures, [])


class ApiReferenceContentTest(unittest.TestCase):
    """The E0 gaps, asserted against the committed document.

    These are the tests that would have caught the original failure. They read
    the committed documents because those are the artifacts a game author gets:
    a generator that extracts correctly and renders nothing is still a document
    that cannot be written from.

    `reference` is the game document and `testing` its other half; assertions
    about a *game's* vocabulary read the first, and anything about the size of
    the surface reads both (ADR-0025).
    """

    @classmethod
    def setUpClass(cls):
        root = Path(__file__).resolve().parents[2]
        cls.reference = (root / "docs/api/jidousha-api.md").read_text(encoding="utf-8")
        cls.testing = (root / "docs/api/jidousha-testing.md").read_text(encoding="utf-8")
        cls.root = root

    def test_the_reference_names_the_rectangle_overlap_helpers(self):
        # E0 hand-wrote overlap arithmetic against a `Rect` that already had
        # `overlaps`, and filed it under "expected to exist and could not find"
        # (F-003). The method existed; the signature did not.
        self.assertIn("pub fn contains(self, point: Vec2) -> bool;", self.reference)
        self.assertIn("pub fn overlaps(self, other: Rect) -> bool;", self.reference)

    def test_the_reference_lists_the_keys_a_game_reaches_for_first(self):
        # E0 gave up on Escape, the digits and `P` rather than play
        # compile-error roulette, and guessed the arrows (F-002).
        for key in ("Escape", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
                    "Digit1", "Space", "ShiftLeft", "F1", "Backquote"):
            self.assertIn(key, self.reference, key)

    def test_every_key_the_source_declares_reaches_the_document(self):
        # The sync guard, parsed by a different route from the generator's, so
        # a change to the macro's shape that the generator stops understanding
        # fails here rather than shipping a short keyboard.
        source = (self.root / "crates/jidousha-input/src/key.rs").read_text(encoding="utf-8")
        body = source.split("keys! {", 1)[1].split("\n}", 1)[0]
        declared = set(re.findall(r"(\w+)\s*=\s*\d+", body))
        self.assertGreater(len(declared), 80, "the invocation itself did not parse")
        block = self.reference.split("pub enum Key {", 1)[1].split("\n}", 1)[0]
        listed = set(re.findall(r"\b([A-Z]\w*)\b", block))
        self.assertEqual(declared - listed, set(), "keys missing from the reference")

    def test_the_reference_gives_every_draw_verb_its_argument_order(self):
        # "I could not have made the first ctx.rect call, because nothing
        # states its argument order" (F-001). `text` included, whose depth
        # lives inside TextStyle rather than trailing — the asymmetry E0 found
        # by trying, and ADR-0018 now records.
        self.assertIn("fn rect(&mut self, rect: Rect, color: Color, depth: Depth);", self.reference)
        self.assertIn(
            "fn circle(&mut self, center: Vec2, radius: f32, color: Color, depth: Depth);",
            self.reference,
        )
        self.assertIn(
            "fn line(&mut self, from: Vec2, to: Vec2, thickness: f32, color: Color, depth: Depth);",
            self.reference,
        )
        self.assertIn("fn text(&mut self, at: Vec2, text: &str, style: TextStyle);", self.reference)

    def test_the_reference_states_the_game_configs_fields_and_the_tick_rate(self):
        # E0 made a gameplay decision out of ignorance about the window size,
        # and recovered 60 Hz from arithmetic in another example's assertion
        # rather than from this document (F-004).
        for field in ("pub title:", "pub seed:", "pub fixed_dt:"):
            self.assertIn(field, self.reference, field)
        self.assertIn("Seconds(1.0 / 60.0)", self.reference)

    def test_the_reference_states_the_cameras_viewport_and_its_default(self):
        # Four separate E0 questions, all answered by one block: viewport
        # exists, Camera is Copy, the default is 1280x720 (F-006).
        self.assertIn("pub viewport: PhysicalSize", self.reference)
        self.assertIn("PhysicalSize::new(1280, 720)", self.reference)

    def test_the_reference_gives_message_its_full_signature(self):
        # E0 copied the four-argument shape out of an example and still did not
        # know whether there was a fifth (F-005).
        self.assertIn(
            "pub fn message(what: &str, specifics: &str, likely_cause: &str, fix: &str) -> String;",
            self.reference,
        )

    def test_the_reference_documents_the_maths_a_game_writes_unqualified(self):
        # `Radians` and `sin_cos` are prelude items. Before the module's members
        # were rendered they appeared only incidentally, inside other
        # signatures, and never as entries of their own.
        self.assertIn("pub fn sin_cos(angle: Radians) -> (f32, f32);", self.reference)
        self.assertIn("#### `Radians`", self.reference)

    def test_the_reference_documents_how_a_game_reaches_a_resource(self):
        # E0 run 2's largest gap. `World` documented seventeen methods and not
        # one of them was about resources, while the Quickstart on the same
        # page called three of the five that were missing (F-016). The cause
        # was the generator, not the doc comments, so this reads the artifact.
        for method in (
            "pub fn insert_resource<T: Resource>(&mut self, value: T);",
            "pub fn remove_resource<T: Resource>(&mut self);",
            "pub fn resource<T: Resource>(&self) -> &T;",
            "pub fn resource_mut<T: Resource>(&mut self) -> &mut T;",
            "pub fn find_resource<T: Resource>(&self) -> Option<&T>;",
            "pub fn find_resource_mut<T: Resource>(&mut self) -> Option<&mut T>;",
        ):
            self.assertIn(method, self.reference, method)

    def test_the_reference_says_which_types_are_resources_and_who_installs_them(self):
        # E0 run 2's parent finding: `Camera` and `Time` were documented as
        # ordinary structs, so nothing said how a game gets one or whether one
        # exists if it never asks (F-021). `Camera` is the trap — a headless run
        # installs none, so a check reading it panics where the window would not.
        for phrase in (
            "held as a world resource",
            "| `Time` |",
            "| `Rng` |",
            "| `Input` |",
            "| `Camera` |",
            "| `Assets` |",
        ):
            self.assertIn(phrase, self.reference, phrase)

    def test_the_reference_states_the_query_shapes_rather_than_only_showing_them(self):
        # The run restructured its whole game around 2-tuples to avoid finding
        # out what would compile (F-023). Arity, the one-tuple and the
        # entity-first yield are the three facts it could not get.
        self.assertIn("tuple of up to six", self.reference)
        self.assertIn("With<T>", self.reference)
        self.assertIn("Without<T>", self.reference)

    def test_the_reference_documents_vec2_rather_than_pointing_at_another_crate(self):
        # "Also in `math`, re-exported from `glam` and documented there" sat
        # under a document opening "if something you want is not here, it is not
        # part of v1" (F-018). The entry is now a compiled example.
        self.assertIn("length_squared", self.reference)
        self.assertIn("Vec2::splat", self.reference)
        self.assertIn("Vec2::ZERO", self.reference)

    def test_the_reference_does_not_shrink(self):
        # A floor, not an exact count: ordinary API growth must not churn this,
        # but a parser regression that halves the output has to fail loudly —
        # and it would be invisible in the diff of a document this size.
        #
        # Counted over **both** documents, because the surface is what must not
        # shrink and it now lives in two files. A floor on the game document
        # alone would be satisfied by a bug that emitted the testing reference
        # nowhere at all.
        whole = self.reference + self.testing
        self.assertGreater(whole.count("pub fn "), 150)
        self.assertGreater(whole.count("#### `"), 80)


class ApiCoverageTest(unittest.TestCase):
    def test_the_item_list_comes_from_the_facade(self):
        self.assertEqual(
            api_coverage.facade_items(FACADE),
            ["App", "Camera", "Draw", "headless", "math", "Sprite"],
        )

    def test_an_item_named_in_an_example_is_covered(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "game.rs"
            source.write_text("let s = Sprite::new(handle);\n")
            self.assertEqual(api_coverage.uncovered(["Sprite"], [source]), [])

    def test_an_item_no_example_names_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "game.rs"
            source.write_text("let s = 1;\n")
            self.assertEqual(api_coverage.uncovered(["Sprite"], [source]), ["Sprite"])

    def test_a_partial_word_does_not_count_as_coverage(self):
        # `Rect` must not be matched by `Rectangle`, or the check passes on
        # items nothing demonstrates.
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "game.rs"
            source.write_text("struct Rectangle;\n")
            self.assertEqual(api_coverage.uncovered(["Rect"], [source]), ["Rect"])

    def test_an_exempt_item_is_not_required_to_appear(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "game.rs"
            source.write_text("nothing\n")
            self.assertEqual(api_coverage.uncovered(["Submit"], [source]), [])

    def test_every_exemption_carries_a_reason(self):
        # The reason is the whole value of the list: without one, an exemption
        # is indistinguishable from giving up on an item.
        for item, reason in api_coverage.EXEMPT.items():
            self.assertTrue(reason.strip(), f"{item} has no reason")

    def test_an_example_naming_an_internal_crate_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "game.rs"
            source.write_text("use jidousha::prelude::*;\nuse jidousha_core::World;\n")
            found = api_coverage.facade_breaches([source])
            self.assertEqual([(line, crate) for _, line, crate in found], [(2, "jidousha_core")])

    def test_the_facade_itself_is_not_mistaken_for_an_internal_crate(self):
        # `jidousha::prelude` contains no internal crate name, and a check that
        # thought otherwise would fail on every correct example.
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "game.rs"
            source.write_text("use jidousha::prelude::*;\nuse jidousha::testing::InputScript;\n")
            self.assertEqual(api_coverage.facade_breaches([source]), [])

    def test_the_committed_tree_is_covered_and_reaches_past_nothing(self):
        self.assertEqual(api_coverage.main(["check-api-coverage"]), 0)

    def test_a_breach_makes_the_whole_run_fail(self):
        # The wiring between finding a problem and saying so with a non-zero
        # exit. Every check above could pass while the script returned 0 and CI
        # stayed green — which is exactly the escape the last three milestones
        # each produced.
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            facade = repo / "crates/jidousha/src"
            facade.mkdir(parents=True)
            (facade / "lib.rs").write_text(
                "// --- ECS ---\npub use jidousha_core::{World};\npub mod prelude {}\n"
            )
            examples = repo / "crates/jidousha/examples"
            examples.mkdir(parents=True)
            (examples / "game.rs").write_text("use jidousha_core::World;\n")
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = api_coverage.main(["check-api-coverage", "--root", str(repo)])
            self.assertEqual(code, 1)
            self.assertIn("reaches past the facade", err.getvalue())

    def test_a_run_with_nothing_to_check_is_a_tooling_fault(self):
        with tempfile.TemporaryDirectory() as directory:
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = api_coverage.main(["check-api-coverage", "--root", directory])
            self.assertEqual(code, 2)
            self.assertIn("found nothing to check", err.getvalue())


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
