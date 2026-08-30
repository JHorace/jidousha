# SANITATION — handoff template for a maintenance session

Copy this file into the handoff, fill every slot, delete the guidance in
parentheses. A sanitation session is **dispatched by evidence, not by a
calendar** and not by a feeling that the repo looks untidy: something in a
FINDINGS ledger, or a wave gate that failed, said a specific thing has rotted.
If the evidence slot below cannot be filled, the pass is not owed yet.

Why this has a template at all: this repo is read by agents, who copy patterns
and act on prose as ground truth (agent-practices, meta-principle 3). Rot here
steers future code, so maintenance is real work — and unfenced maintenance is
how a "tidy-up" quietly becomes a behavior change nobody reviewed.

---

## 1. Pass type — pick exactly one

| Type | The question it asks | Touches | Mode |
|---|---|---|---|
| **doc-truth audit** | Do this document's checkable claims still match the code? | docs (and the code only to read it) | judgment · report-first |
| **exemplar audit** | Is the most-copyable code still the pattern we want copied? | examples, worked paths, template code | judgment · report-first · transcript-identical |
| **dead-weight sweep** | What is mechanically unreachable, unused, or broken-by-link? | code, deps, assets, links | mechanical · transcript-identical |
| **history-bleed sweep** | Has a living document turned into archaeology? | docs (living ones), ADRs, git history | judgment · report-first |

**Selected type:** …

**Never two types in one session, and never judgment plus mechanical.** They
fail in opposite directions: a mechanical sweep is safe exactly because nobody
is exercising taste in it, and a judgment pass is worth having exactly because
somebody is. Run together, the diff stops being reviewable — a taste change
hides inside a lint fix, and a genuinely needed lint fix gets argued about as
if it were taste. Two passes are two sessions.

## 2. Ledger evidence — what dispatched this pass

**This pass is dispatched by these FINDINGS entries:**

- (G-NNN / F-NNN — `<ledger file>` — one line on what it says has rotted)
- …

**Or, if the dispatch is a failed gate:** (which gate, which run, what it
returned — and why the failure indicts a *class* of thing rather than one bug,
because one bug is a fix, not a pass.)

**Or, if this is owner policy:** (say so in exactly those words, with the date
of the approval. A pass with no ledger entry and no gate behind it is the
owner's call to make and nobody else's — do not infer one.)

## 3. Scope fence

- **In scope:** (the named files, documents, or crates; be exhaustive — a
  sanitation session's scope is a list, never a description.)
- **Out of scope:** (everything else, and specifically anything another
  in-flight session owns. Name the sessions.)
- **Sessions in flight while this one runs:** (list, with what each owns.)

## 4. The fences for the selected type

Keep the block for the type selected in §1; delete the others.

### Report-first (every judgment pass)

The session produces its **findings first**, as a written report, and changes
nothing until the report exists. The report names, per item: what it looked at,
what it found, what it proposes, and the evidence. Then, and only then, the
session applies the items the fence below allows. A judgment pass that starts
by editing has no way to tell a reviewer what it *considered* and left alone,
and "considered and left alone" is most of the value of the pass.

- A proposal the report cannot justify from evidence is not applied; it is
  listed as a question for the owner.
- Deleting working guidance because it reads dated is out of bounds. Anything
  that dies, dies with a cited reason in the report.

### Transcript-identical (every code-touching pass)

The claim "this changed nothing" is machine-checkable here, so make it:

- Capture the relevant `--verify` transcripts and `target/verify/` reports
  **before** the first edit.
- After: the same runs produce **byte-identical transcripts** and identical
  verdicts, seeds and stamps. A diff is a behavior change and the change is
  reverted, not explained.
- If a change genuinely must alter a transcript, it is not sanitation. Stop and
  hand it back as its own piece of work.

### Docs-only passes

No code diff at all. A code diff outside the scope fence is a defect in the
session, not a bonus.

## 5. Definition of done

1. CLAUDE.md's definition of done, in full.
2. `tools/doctor` `ENV_OK`; `tools/test` green with `target/verify/report.json`
   as the verdict.
3. The type's fence, demonstrated rather than asserted: the report exists
   (judgment passes) and/or the before/after transcripts are identical
   (code-touching passes). Put the evidence in the PR.
4. Every change traces to a source in the PR: a ledger G-/F-number, the gate
   that failed, or the words "owner policy" with the approval date. No silent
   judgment calls.
5. Everything considered and deliberately left alone is listed, with why. This
   is the item that makes the next pass cheaper.
6. Findings the pass itself generated are filed in the ledger it read from —
   including the case where a document **misled** this session, which is a
   finding in its own right (agent-practices §2.5).
7. The closing note names the owner-loop actions triggered: "Sync now" (with
   the Sync-button quirk), any playtest owed, and whether a further pass is
   indicated — with its evidence, or the honest statement that there is none.
