# Tracker Ladder: project plan

This is the source of truth for scope and milestones. Agents work one milestone at a time (see `docs/milestone-prompt.txt`) and tick its box here when it's done. "Tracker Ladder" and the `ladder-*` crate names are working names.

## Context
Most private-tracker users settle on lower-tier trackers, and the top trackers take years of ladder-climbing to reach. Users need two answers: **which trackers to focus on** and **whether the next step up is worth the grind**. trackerpathways.org already maps invite routes but has no community quality signal. This site adds community voting on trackers, built on top of that route data.

Stack: Rust 2024 workspace + Postgres (`sqlx`), Topcoat SSR frontend.

### Decisions
- **Identity:** anonymous votes are allowed but carry a reduced weight. Discord and Reddit OAuth accounts get full weight. No email or raw IP is stored.
- **Votes:**
  - **Primary:** per-tracker ratings, 1–5 on several axes.
  - **Secondary:** a pairwise "X vs Y" page for users who want to give finer-grained rankings.
- **"What next?" planner:** a later phase, recorded under Future work below.
- **Hosting:** self-hosted with Docker Compose (app + Postgres) behind your reverse proxy.

### Seed data source
trackerpathways.org sits behind a Cloudflare challenge, but the project is open source under the **MIT** licence at `github.com/handokota/trackerpathways`. Its `src/data/trackers.json` contains:
- `trackerInfo`: 182 trackers, each with abbreviation, type, country, founding year and description.
- `routeInfo`: 598 invite routes (source → target, `days`, `reqs`, `active`, `updated`).
- `unlockInviteClass`: for 60 trackers, the days and requirements to reach invite rank.

We vendor this file with attribution, together with the commit SHA it came from.

Second source: the r/TrackersInfo "official recruitments" wiki (trackerpathways links to it). It goes in as a link on tracker pages, not as scraped data.

## Architecture
A Cargo workspace (edition 2024) with tooling copied from **zayden**, the newest and primary exemplar (last commit 2026-10-18).

```
crates/core/   (ladder-core) pure domain: types, category normalisation, vote weighting,
               Bayesian aggregation, Bradley–Terry, tier assignment (no IO)
crates/admin/  (ladder-admin) CLI: `import` (trackerpathways JSON → DB, idempotent upsert),
               `recompute`, `void-voter <id>`
web/           Topcoat SSR app: pages, OAuth, sessions, forms, sqlx queries
migrations/    paired up/down SQL
data/trackerpathways/{trackers.json,LICENSE,SOURCE}   vendored seed + attribution
```

### Frontend: Topcoat SSR (0.10.0), as in zayden `web/`
- Server-rendered forms are enough for rating and pairwise voting, and SSR is good for SEO.
- It already has a working Discord OAuth + session-cookie module to reuse:
  - `zayden/web/src/auth/`
  - `zayden/web/src/state.rs` (moka)
  - `zayden/web/src/session_pruning.rs`
- Route groups follow `zayden/web/src/router.rs` (`ROUTE_GROUPS` fold).
- Components come from the `topcoat ui` CLI (`web/components.toml`).
- Tailwind is built via `web/build.rs`.
- Topcoat is pre-1.0, so verify every API against `zayden/docs/topcoat/` (checked against 0.10.0) rather than from memory.

### Tooling to copy from zayden
- Root `Cargo.toml` `[workspace.lints]` (unwrap and expect denied).
- `rustfmt.toml`, `clippy.toml`, `deny.toml`, `bacon.toml`.
- `.github/workflows/ci.yml`: fmt (nightly), clippy, test with a postgres:18 service, cargo-deny, and `cargo sqlx prepare --check`.

### Patterns to copy from afterglow
- `Config::from_lookup` (`afterglow/server/src/platform/config.rs`).
- Per-group governor rate limits (`platform/rate_limit.rs`).
- cargo-chef Dockerfile (`docker/Dockerfile.server`).
- `compose.yaml` with a Postgres healthcheck, `${X:?}` required env vars and a port bound to 127.0.0.1.

Record all of the above in `.claude/REFERENCES.md`.

## Data model (Postgres)
- **`trackers`:** `id, slug, name, abbr, categories text[]` (normalised), `raw_type, country, founded_year, description, unlock_days, unlock_reqs, source_sha, updated_at`.
  - `listed = false` marks a stub: a route endpoint that upstream's `trackerInfo` does not describe. Stubs have no info columns and cannot be rated.
- **`routes`:** `source_id, target_id, days, reqs, active, updated_label`. PK is (source, target).
- **`accounts`:** `id, provider ('discord'|'reddit'), provider_subject, display_name, provider_created_at, created_at`.
  - No email.
  - Provider account age feeds the vote weight.
- **`sessions`:** follows zayden's pattern, but keyed on a SHA-256 `token_hash` rather than the raw token.
- **`anon_voters`:** `id, fingerprint_hmac, created_at`.
  - The HMAC is keyed with a server secret over a long-lived cookie id, the /24 or /48 IP prefix and the user agent.
  - Raw IPs and user agents are never stored.
- **`memberships`:** `voter, tracker_id, status ('current'|'former')`.
  - Membership is self-declared.
  - A voter can only rate trackers they are or were on.
- **`ratings`:** `voter, tracker_id, axis, score smallint 1–5, updated_at`.
  - Unique per (voter, tracker, axis), so re-voting overwrites.
  - Axes: **overall, content, community, retention/ratio ease, longevity/stability**.
- **`pairwise_votes`:** `voter, winner_id, loser_id, created_at`.
- **`tracker_scores` / `pairwise_scores`:** materialised aggregates, refreshed by a background task and by `admin recompute`.
- **`voter`:** a tagged pair of (`account_id` | `anon_id`), implemented as a `voters` table with a `kind` column so foreign keys stay simple.

## Scoring (`crates/core`, pure and unit-tested)
- **Weights** (configurable):
  - OAuth account: 1.0.
  - OAuth account younger than 30 days on its provider: 0.5.
  - Anonymous: 0.25.
  - Anonymous votes are capped at 30% of a tracker's total weight.
- **Per-axis score:** weighted Bayesian average `(C·m + Σw·s) / (C + Σw)`.
  - `m` is the global axis mean and `C` is the confidence constant.
  - Each score is shown with its effective vote count.
- **Tiers (S–E):** assigned by percentile of the overall score among trackers that have enough confidence. Trackers without enough votes are shown as "unranked".
- **Pairwise:** Bradley–Terry fitted with MM iterations over the weighted votes and shown on its own page.
  - Matchups are drawn from trackers the voter is a member of, or from within one category.
- **Categories:** trackerpathways' free-text `type` is mapped onto a fixed set: Movies/TV, Music, Books/Audiobooks, Anime, Games/Software, General, E-Learning, Adult, Regional. The mapping table lives in core and has tests.

## Pages
- **`/`:** rankings table.
  - Category filter, sort by any axis, tier badges, confidence indicators.
- **`/t/{slug}`:**
  - Axis scores and the rating form, which appears once membership is declared.
  - Invite-rank requirements.
  - **"Next steps from here":** outgoing routes, each with the target's community score, days to unlock and requirements. This is the cheap first answer to "is the next tracker worth it" before the full planner exists.
  - Incoming routes.
  - Link to the official recruitments wiki.
- **`/compare`:** pairwise voting, plus the Bradley–Terry ranking.
- **`/login`:** Discord and Reddit buttons. Callback routes are `/auth/{provider}/callback`; `/logout` ends the session.
- **`/me`:** memberships, my votes, and **delete my data**.
- **`/about`:** methodology, vote weights, data attribution (trackerpathways MIT) and privacy (essential cookies only).

## Milestones
One milestone per session and per small set of commits. Never commit without the user's approval. A milestone is done when its checks pass and its "Done when" holds.

- [x] **M1: Scaffold and docs.**
  - Workspace, tooling copied from zayden, `.env.example`, `.gitignore`, README, `CLAUDE.md`, `.claude/REFERENCES.md`, `docs/topcoat/`.
  - A minimal Topcoat `web` with a home page and a 404 page. CI covers fmt, clippy, test and deny.
- [x] **M2: Schema and import.**
  - Deviation: 19 route endpoints missing from `trackerInfo` are imported as stub trackers (`listed = false`). Sessions store a 32-byte `token_hash`, not the raw token. `categories` stays empty until M3's normalisation.
  - Vendor `data/trackerpathways/{trackers.json,LICENSE,SOURCE}`. `SOURCE` records the upstream commit SHA and date. Fetch the file with `gh api repos/handokota/trackerpathways/contents/src/data/trackers.json`, not the Cloudflare-protected site.
  - Paired up/down migrations for every table in the data model.
  - `crates/admin` with `import` (idempotent upsert keyed on tracker name, never touches votes).
  - Add the postgres service and the `sqlx prepare --check` job to CI, and commit `.sqlx/`.
  - **Done when:** a `sqlx::test` shows the import, run twice, gives 182 trackers and 598 routes. Recount if upstream changed, and record the counts in `SOURCE`.
- [ ] **M3: Core logic (`ladder-core`).**
  - Category normalisation table, vote weights and anonymous-vote cap, Bayesian axis score, percentile tiers with an "unranked" threshold, Bradley–Terry (MM).
  - All of it is pure and configurable through a `ScoringConfig` struct.
  - **Done when:** tests in `crates/core/tests/` cover Bayesian edge cases (no votes, one vote), the anonymous cap, Bradley–Terry converging on a fixture with a known order, and category mapping of every distinct upstream `type` token.
- [ ] **M4: Read-only site.**
  - `/` rankings (category filter, sort by axis, tier badges, confidence). Until M6 there are no votes, so every tracker shows "unranked".
  - `/t/{slug}`: details, invite-rank requirements, outgoing "next steps" routes with days and requirements, incoming routes, official-recruitments wiki link.
  - `/about`: methodology, attribution and privacy.
  - **Done when:** router tests confirm `/t/aither` lists Anthelion with "Prometheus+", and an unknown slug returns 404.
- [ ] **M5: Identity.**
  - Discord OAuth (adapted from `zayden/web/src/auth/`) and Reddit OAuth (scope `identity`), with the provider account age stored.
  - Sessions with pruning.
  - Anonymous voter cookie plus fingerprint HMAC: never store raw IPs or user agents.
  - Rate limits on the write routes.
  - Origin policy (CSRF).
  - **Done when:** tests use fake OAuth providers behind a trait. The real apps get a manual check with the user's credentials.
- [ ] **M6: Voting.**
  - Membership declaration, then 1–5 ratings on the five axes, with re-voting overwriting the previous vote.
  - A background refresh of `tracker_scores`, plus `admin recompute` and `admin void-voter`.
  - `/me`: memberships, my votes and "delete my data".
  - **Done when:** tests show an anonymous vote carries 0.25 weight, re-voting overwrites, and deletion cascades.
- [ ] **M7: Pairwise.**
  - `/compare` shows matchups drawn from the voter's memberships or from one category.
  - The Bradley–Terry ranking is refreshed into `pairwise_scores`.
  - **Done when:** a vote is recorded and the ranking page renders from the fitted scores.
- [ ] **M8: Deployment.**
  - `docker/Dockerfile.web` (cargo-chef, Tailwind with checksum, `topcoat asset bundle --release`, non-root).
  - `compose.yaml`: postgres:18 with a healthcheck, `${X:?}` required env vars, port bound to 127.0.0.1.
  - CI image job.
  - **Done when:** `docker compose up` followed by `admin import` serves the site locally, verified in the browser.

## Future work (not in this build)
- **"What should I do next?" planner:**
  - The user enters the trackers they're on and the categories they care about.
  - The site ranks reachable targets by the community-score gain per day of grind, using `routes.days` plus `unlock_days`, over multi-hop paths with Dijkstra.
  - It also flags dead or inactive routes.
- Crowd-sourced "actual time to unlock" reports per route, to correct the official day counts.
- Optional tier-list drag-and-drop submissions.
- A scheduled re-import from the upstream trackerpathways repo, with a diff review.

## Verification
- `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`, `cargo deny check`.
- `cargo test --workspace`:
  - Core unit tests: Bayesian edge cases, weight caps, Bradley–Terry convergence on a known fixture, category mapping.
  - `sqlx::test` integration tests against a throwaway Postgres: running the import twice is idempotent and gives 182 trackers / 598 routes; a re-vote overwrites; account deletion cascades.
- `docker compose up`, then run `admin import`. Check in the built-in browser:
  - The rankings page renders.
  - A tracker page shows the expected routes; for example Aither → Anthelion should read "Prometheus+".
  - Voting works with an anonymous vote. The weight difference can be checked in the DB.
  - OAuth is tested against your real Discord/Reddit dev apps. You create those and fill in `.env`.
- `cargo sqlx prepare --check` runs in CI.
