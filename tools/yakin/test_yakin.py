"""Behavioral tests for tools/yakin/yakin — the queue schema, the window, the lease, the roles.

Run:  python3 -m unittest discover -s tools/yakin -p 'test_*.py'

Not a `tools/test` phase: yakin is owner-side process tooling outside the
engine's gates, and its tick runs these itself (DOCTRINE.md §1, Startup).

Key functions: `load_yakin`.
Depends on: the Python 3.9+ standard library, and `git` for the end-to-end case.
"""

from __future__ import annotations

import contextlib
import importlib.machinery
import importlib.util
import io
import json
import subprocess
import sys
import tempfile
import unittest
from datetime import datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load_yakin():
    loader = importlib.machinery.SourceFileLoader("yakin_tool", str(HERE / "yakin"))
    spec = importlib.util.spec_from_loader("yakin_tool", loader)
    module = importlib.util.module_from_spec(spec)
    # dataclasses resolve their module through sys.modules.
    sys.modules["yakin_tool"] = module
    loader.exec_module(module)
    return module


yakin = load_yakin()

SPEC = "# t\n\n## Goal\nx\n\n## Fence\nx\n\n## Done when\nx\n"


def task(**overrides):
    row = {"id": "t-one", "title": "A task", "kind": "system", "variant": "V1",
           "size": "S", "spec": "tools/yakin/tasks/t-one.md", "status": "queued"}
    row.update(overrides)
    return row


def checkpoint(stage="implement", tick="live", resumes="0"):
    return (f"# CHECKPOINT — t-one\n\ntask: t-one\nvariant: V1\nstage: {stage}\n"
            f"tick: {tick}\nresumes: {resumes}\ngates-green-at: none\n\n## Done so far\n")


class QueueSchema(unittest.TestCase):
    def test_the_committed_queue_validates_against_its_documented_schema(self):
        self.assertEqual(yakin.command_check(HERE.parent.parent), 0)

    def test_a_claim_field_in_the_queue_is_rejected_because_claims_live_on_branches(self):
        errors = yakin.validate_queue({"schema": 1, "tasks": [task(claimed_by="tick-3")]},
                                      {"tools/yakin/tasks/t-one.md": SPEC})
        self.assertTrue(any("claim state lives on branches" in e for e in errors), errors)

    def test_a_status_other_than_queued_is_rejected(self):
        errors = yakin.validate_queue({"schema": 1, "tasks": [task(status="claimed")]},
                                      {"tools/yakin/tasks/t-one.md": SPEC})
        self.assertTrue(any("status" in e for e in errors), errors)

    def test_a_spec_missing_its_fence_section_is_rejected(self):
        errors = yakin.validate_queue({"schema": 1, "tasks": [task()]},
                                      {"tools/yakin/tasks/t-one.md": "## Goal\n## Done when\n"})
        self.assertTrue(any("## Fence" in e for e in errors), errors)

    def test_a_large_task_may_not_run_outside_the_window(self):
        errors = yakin.validate_queue({"schema": 1, "tasks": [task(size="L", window="any")]},
                                      {"tools/yakin/tasks/t-one.md": SPEC})
        self.assertTrue(any("window `any`" in e for e in errors), errors)

    def test_duplicate_ids_are_rejected(self):
        errors = yakin.validate_queue({"schema": 1, "tasks": [task(), task()]},
                                      {"tools/yakin/tasks/t-one.md": SPEC})
        self.assertTrue(any("not unique" in e for e in errors), errors)


class Window(unittest.TestCase):
    def test_the_window_opens_wednesday_at_one_and_closes_thursday_at_four_thirty(self):
        # 2026-10-07 is a Wednesday, 2026-10-08 a Thursday.
        cases = {
            datetime(2026, 10, 7, 0, 59): False,
            datetime(2026, 10, 7, 1, 0): True,
            datetime(2026, 10, 7, 23, 59): True,
            datetime(2026, 10, 8, 4, 29): True,
            datetime(2026, 10, 8, 4, 30): False,
            datetime(2026, 10, 6, 12, 0): False,
        }
        for moment, expected in cases.items():
            self.assertEqual(yakin.in_window(moment), expected, moment)


class Lease(unittest.TestCase):
    def test_a_task_with_no_branch_is_free(self):
        self.assertEqual(yakin.classify(None, merged=False)[0], "free")

    def test_a_branch_committed_to_within_ninety_minutes_is_held_by_a_live_tick(self):
        branch = yakin.Branch(age_minutes=89, checkpoint=checkpoint(), has_blocked=False)
        self.assertEqual(yakin.classify(branch, merged=False)[0], "live")

    def test_a_silent_live_branch_past_ninety_minutes_is_resumable(self):
        branch = yakin.Branch(age_minutes=90, checkpoint=checkpoint(), has_blocked=False)
        self.assertEqual(yakin.classify(branch, merged=False)[0], "stale")

    def test_a_released_branch_is_continued_at_once_without_counting_a_resume(self):
        branch = yakin.Branch(age_minutes=5, checkpoint=checkpoint(tick="released"), has_blocked=False)
        self.assertEqual(yakin.classify(branch, merged=False)[0], "ready")

    def test_a_third_resume_becomes_a_blocked_write_instead(self):
        branch = yakin.Branch(age_minutes=200, checkpoint=checkpoint(resumes="2"), has_blocked=False)
        self.assertEqual(yakin.classify(branch, merged=False)[0], "exhausted")

    def test_done_blocked_and_merged_tasks_are_never_taken(self):
        done = yakin.Branch(age_minutes=500, checkpoint=checkpoint(stage="done"), has_blocked=False)
        blocked = yakin.Branch(age_minutes=500, checkpoint=checkpoint(), has_blocked=True)
        self.assertEqual(yakin.classify(done, merged=False)[0], "done")
        self.assertEqual(yakin.classify(blocked, merged=False)[0], "blocked")
        self.assertEqual(yakin.classify(None, merged=True)[0], "merged")

    def test_started_work_is_taken_before_fresh_work_and_the_oldest_first(self):
        rows = [
            row("fresh", 0, "free"),
            row("young", 1, "stale", age=100.0),
            row("old", 2, "stale", age=300.0),
        ]
        chosen, _ = yakin.pick(rows, inside=True, role="implement")
        self.assertEqual(chosen["id"], "old")

    def test_outside_the_window_only_an_any_window_task_is_taken(self):
        rows = [row("big", 0, "free"), row("canary", 1, "free", window="any")]
        self.assertEqual(yakin.pick(rows, inside=False, role="implement")[0]["id"], "canary")
        self.assertIsNone(yakin.pick(rows[:1], inside=False, role="implement")[0])


def row(task_id, index, state, role="implement", window="burn-down", age=0.0):
    return {"id": task_id, "index": index, "state": state, "role": role,
            "window": window, "age_minutes": age}


class Roles(unittest.TestCase):
    def test_an_unclaimed_split_task_is_design_work_and_an_unclaimed_v1_task_is_implement_work(self):
        self.assertEqual(yakin.role_of("V2", None), "design")
        self.assertEqual(yakin.role_of("V3", None), "design")
        self.assertEqual(yakin.role_of("V1", None), "implement")

    def test_a_split_task_belongs_to_the_implementer_once_its_checkpoint_says_implement(self):
        self.assertEqual(yakin.role_of("V2", "design"), "design")
        self.assertEqual(yakin.role_of("V2", "implement"), "implement")

    def test_a_v1_task_never_has_a_design_stage_even_while_its_checkpoint_reads_design(self):
        self.assertEqual(yakin.role_of("V1", "design"), "implement")

    def test_each_role_is_handed_only_its_own_rows(self):
        rows = [row("v2-fresh", 0, "free", role="design"), row("v1-fresh", 1, "free")]
        self.assertEqual(yakin.pick(rows, inside=True, role="design")[0]["id"], "v2-fresh")
        self.assertEqual(yakin.pick(rows, inside=True, role="implement")[0]["id"], "v1-fresh")

    def test_a_role_with_nothing_of_its_own_gets_none_even_when_the_other_role_has_work(self):
        rows = [row("v2-fresh", 0, "free", role="design")]
        chosen, reason = yakin.pick(rows, inside=True, role="implement")
        self.assertIsNone(chosen)
        self.assertEqual(reason, "no claimable implement work")

    def test_a_stale_branch_of_the_role_is_continued_before_its_unclaimed_tasks(self):
        rows = [
            row("v2-fresh", 0, "free", role="design"),
            row("v2-dead-designer", 1, "stale", role="design", age=120.0),
            row("v2-dead-implementer", 2, "stale", age=400.0),
        ]
        self.assertEqual(yakin.pick(rows, inside=True, role="design")[0]["id"], "v2-dead-designer")
        self.assertEqual(yakin.pick(rows, inside=True, role="implement")[0]["id"], "v2-dead-implementer")

    def test_next_without_a_role_prints_usage_and_exits_nonzero(self):
        for argv in (["yakin", "next"], ["yakin", "next", "--role"], ["yakin", "next", "--role", "review"]):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = yakin.main(argv)
            self.assertEqual(code, 2, argv)
            self.assertIn("usage:", out.getvalue())


class EndToEnd(unittest.TestCase):
    def test_next_reads_the_queue_from_the_default_branch_and_claims_from_remote_branches(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            origin, clone = tmp / "origin.git", tmp / "clone"

            def run(*args, cwd=clone):
                subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True)

            run("git", "init", "--bare", "-b", "main", str(origin), cwd=tmp)
            run("git", "clone", str(origin), str(clone), cwd=tmp)
            run("git", "config", "user.email", "t@example.com")
            run("git", "config", "user.name", "t")
            queue = {"schema": 1, "tasks": [task(window="any"),
                                            task(id="t-two", spec="tools/yakin/tasks/t-two.md", window="any"),
                                            task(id="t-three", spec="tools/yakin/tasks/t-three.md",
                                                 variant="V2", window="any")]}
            (clone / "tools/yakin/tasks").mkdir(parents=True)
            (clone / "tools/yakin/queue.json").write_text(json.dumps(queue))
            run("git", "add", "-A")
            run("git", "commit", "-m", "queue")
            run("git", "push", "origin", "main")
            run("git", "checkout", "-b", "claude/yakin-t-one")
            run_dir = clone / "tools/yakin/runs/t-one"
            run_dir.mkdir(parents=True)
            (run_dir / "CHECKPOINT.md").write_text(checkpoint())
            run("git", "add", "-A")
            run("git", "commit", "-m", "claim")
            run("git", "push", "origin", "claude/yakin-t-one")
            run("git", "fetch", "--prune", "origin",
                "+refs/heads/claude/yakin-*:refs/remotes/origin/claude/yakin-*")

            def next_for(role):
                out = io.StringIO()
                with contextlib.redirect_stdout(out):
                    code = yakin.command_next(clone, role)
                return code, out.getvalue()

            code, text = next_for("implement")
            self.assertEqual(code, 0, text)
            self.assertRegex(text, r"t-one\s+live")
            self.assertNotIn("t-three", text)
            self.assertTrue(text.strip().endswith("PICK t-two start claude/yakin-t-two"), text)
            code, text = next_for("design")
            self.assertEqual(code, 0, text)
            self.assertNotIn("t-two", text)
            self.assertTrue(text.strip().endswith("PICK t-three start claude/yakin-t-three"), text)


if __name__ == "__main__":
    unittest.main()
