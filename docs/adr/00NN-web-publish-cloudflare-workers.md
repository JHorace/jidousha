# ADR-00NN: Web builds auto-publish to Cloudflare Workers static assets

Status: accepted · 2026-08-16
(NN: assign the next free ADR number when landing this in the repo.)

## Context

Web is a tier-1 target because a playtest link that runs anywhere is the point
(ADR-0005). That promise needs a publishing workflow, and the loop that matters
is shaped by who writes the code: agents open PRs, a human playtests from a
browser before merge. Per-branch preview URLs are therefore the load-bearing
feature, not a nicety. A jidousha web build is purely static (html + js glue +
wasm + assets), single-threaded — no COOP/COEP, no server component.

## Decision

**Cloudflare Workers static assets** is the publish target (the successor to
Cloudflare Pages; static-only via `wrangler deploy` with `assets.directory`,
no worker script).

- `main` deploys to the production URL; every PR branch deploys to its preview
  URL, posted back to the PR as a sticky comment.
- The engine repo publishes its examples (including `prototype_kit`) as
  dogfood. Generated game repos receive the same workflow via a template the
  `make-game` skill copies — every prototype is born with a playtest URL.
- Design and mechanics: `docs/internal/web-publish.md` (W-milestones).

## Rationale

- Preview-URL-per-branch by default matches the agent-PR/human-playtest loop
  exactly; GitHub Pages (the zero-account alternative) structurally lacks it.
- Single-CLI deploys (`wrangler`, API-token auth) are trivially automatable
  from CI; generous free tier; correct wasm MIME; custom headers if ever needed.
- itch.io/butler is a distribution channel, not a preview system — possible
  later addition for public releases, not a substitute.

## Consequences

- Owner-owned setup (agents never create accounts/tokens): Cloudflare account,
  API token, two GitHub secrets — checklist in web-publish.md §7. Missing
  secrets are a BLOCKED-class condition for W1+, not something to work around.
- CI-only dependencies (node + wrangler) stay out of the local toolchain; local
  dev uses `tools/serve-web`. `wasm-bindgen-cli` joins doctor's checks
  (version-matched to the crate — the classic silent breakage).
- Fork PRs don't receive secrets and thus don't get previews — acceptable:
  agents push branches, not forks. Documented, not worked around.

## Alternatives rejected

- **GitHub Pages**: zero new accounts, but one site per repo and no native
  per-PR previews — loses the loop this exists for.
- **Netlify/Vercel**: equivalent previews, tighter free tiers, no
  wasm-specific advantage.
- **itch.io (butler)**: wrong granularity (channels, not branches); revisit as
  a release channel post-v1.
