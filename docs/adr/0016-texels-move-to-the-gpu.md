# ADR-0016: Decoded texels move to the GPU and leave the asset store

Status: accepted · 2026-08-16

## Context

A0 gave `Assets` a slot per asset holding what arrived. A1 filled the texture
slots with decoded `TextureData` — width, height, and `width * height * 4` bytes
of RGBA. R2 is the first milestone with somewhere else to put them.

Two facts decide the shape of the hand-off, and they point the same way:

- **Exactly one thing may read texels.** renderer.md §3 is contract-marked:
  nothing in simulation may depend on texture dimensions or pixels, because a
  game whose behaviour changes when the art is re-exported is a game whose
  replay is not reproducible. The renderer is the only sanctioned reader, and it
  reads each texture once, to upload it.
- **The copy is not small.** The envelope allows 2048×2048 (renderer.md §8),
  which is 16 MB per texture. A prototype with twenty such textures would hold
  320 MB of pixels that nothing is allowed to look at. On the web, where wasm
  linear memory grows and never shrinks, that is the difference between a page
  that loads and one that does not.

assets.md §5 already said "CPU-side pixels are then dropped", without saying by
what mechanism. The mechanism is the decision, because the obvious ones differ
in whether the rule can be forgotten.

## Decision

**`Assets::take_uploads` moves each newly-ready texture's texels out of the
store, once.**

- `commit` queues the `AssetId` of every texture it turns `Ready`, in commit
  order.
- `take_uploads() -> Vec<TextureUpload>` drains that queue, taking the
  `TextureData` out of each slot by value.
- `jidousha-render-core::upload_ready_textures` is the only caller: it calls
  `backend.create_texture` for each and registers the result in the
  `TextureTable`.
- `Assets::texture_of` therefore returns `None` after the hand-off. Its
  documentation says so, and says why.
- The queue holds ids, not data, and waits as long as it has to. A window
  arrives several frames into a program and a headless run never has one, so
  "there is no renderer yet" must not lose anything.

## Rationale

- **Moving makes the drop structural.** The alternative — lend the texels, then
  free them — is two steps where the second can be forgotten, and forgetting it
  fails silently as memory nobody notices until a browser tab dies. A move has
  no second step.
- **One reader, enforced by ownership.** After the upload the store cannot serve
  a texel to anyone, so the §3 contract stops depending on everybody
  remembering it.
- **The queue is on the timeline.** Ids are queued at `commit`, which is the
  single point where readiness changes (assets.md §4). Upload order therefore
  follows commit order, which is recorded and replayable — rather than following
  a walk of the asset table, whose order is an implementation detail. Backend
  texture ids are assigned in upload order, so this is what makes two runs of the
  same script produce the same ids, which R4's golden images will need.
- **Unload wins.** A texture unloaded between the commit that readied it and the
  upload is dropped rather than uploaded: the game said it was finished with it,
  and the generation check on the queued id is what stops a recycled slot
  answering in its place.

## Consequences

- `texture_of` has a state a reader must be told about: `Ready`, but empty.
  That is the cost, and it is why this ADR exists — "why did my texels
  disappear?" is a fair question with a non-obvious answer.
- Headless runs, `tools/verify`, and the asset tests keep reading texels,
  because nothing takes them. That is not an accident of this design; it is what
  makes `examples/load_from_disk.rs` and the A1 tests possible at all.
- **Device loss cannot re-upload.** v1 treats a lost device as fatal
  (renderer.md §10), so nothing depends on re-uploading today. If device
  recreation is ever wanted, the texels are no longer in memory to re-upload
  from, and the honest fix is to re-read them from disk — which is what the
  asset store is for — rather than to keep a permanent shadow copy against an
  event that ends the program anyway.
- A second consumer of texels would have to arrive before the renderer or not at
  all. There is no such consumer in v1's scope, and if one appears (a CPU-side
  collision mask from an alpha channel, say) it wants a different thing anyway:
  a derived product computed at load, not the pixels.

## Alternatives rejected

- **Lend the texels and free them afterwards.** `take_new_textures() ->
  Vec<TextureHandle>` plus `texture_of` plus an explicit free. Three calls where
  one will do, a liveness question at every one of them (a handle can be
  unloaded between the commit and the read, and `texture_of` panics on an
  unloaded handle), and a free step that fails silently when skipped.
- **Keep the texels forever.** Simplest, and `texture_of` stays uniform. It also
  keeps the largest allocations in a program alive for its whole run so that
  nothing may read them, which is the definition of dead weight — and it is
  worst exactly where the engine is weakest, in a browser tab.
- **Push at commit instead of pulling at upload.** `Assets` calls the backend
  itself. It would make the store depend on the renderer, inverting the layering
  the whole crate graph is built on, and there is frequently no backend to call.
- **Reference-count the texels.** Solves a sharing problem the engine does not
  have — v1 has one renderer, no re-upload, and no second reader — at the cost
  of making "when is this freed" a question again (assets.md §1 rejected
  refcounting for asset lifetimes for the same reason).
