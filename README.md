# Tracker Rankings

Community rankings of private trackers, and whether the next rung of the invite ladder is worth the climb.

- **Ratings**: members rate the trackers they're on across five axes (overall, content, community, retention/ratio ease, longevity). Scores use weighted Bayesian averages. Anonymous votes count at reduced weight; Discord and Reddit sign-in votes count fully.
- **Pairwise**: an optional "X vs Y" page feeds a Bradley–Terry ranking.
- **Routes**: invite paths, unlock times and requirements come from [trackerpathways](https://github.com/handokota/trackerpathways) (MIT).

See [`docs/PLAN.md`](docs/PLAN.md) for scope and milestones.

## Layout

| Path                    | Purpose                                         |
| ----------------------- | ----------------------------------------------- |
| `crates/core`           | `rankings-core`: pure scoring and domain logic    |
| `crates/admin`          | `rankings-admin`: import and maintenance CLI      |
| `web`                   | Topcoat SSR site                                |
| `migrations/`           | Postgres migrations (up/down pairs)             |
| `data/trackerpathways/` | Vendored seed data with attribution             |
| `docs/`                 | Plan, Topcoat reference, agent milestone prompt |

## Run locally

Requires the nightly toolchain (pinned), `topcoat` CLI 0.10.0, `sqlx-cli` 0.9, and Docker for Postgres.

```sh
cp .env.example .env                 # set DATABASE_URL and COOKIE_KEY
sqlx migrate run
cargo run -p rankings-admin -- import  # idempotent; never touches votes
topcoat asset bundle -p web
cargo run -p web                     # http://localhost:3000
```

Browse via `localhost`, not `127.0.0.1`: the session cookie is always `Secure`, and browsers accept that over plain HTTP only on `localhost`.

Sign-in needs a Discord and/or Reddit OAuth app. Register `${PUBLIC_BASE_URL}/auth/discord/callback` (Discord, scope `identify`) or `${PUBLIC_BASE_URL}/auth/reddit/callback` (Reddit "web app", scope `identity`), then fill the matching variables in `.env`. A provider with no credentials is hidden from `/login`.

To re-vendor the seed, follow the command in `data/trackerpathways/SOURCE` and update its commit, date, checksum and counts.

Checks: `bacon` (clippy by default), or see the full suite in `CLAUDE.md`.

## Attribution

The tracker and route data is © the trackerpathways contributors, under the MIT License. See `data/trackerpathways/LICENSE`.
