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
cargo run -p rankings-admin -- migrate
cargo run -p rankings-admin -- import  # idempotent; never touches votes
topcoat asset bundle -p web
cargo run -p web                     # http://localhost:3000
```

The web server recomputes `tracker_scores` at startup and a few seconds after each vote (at most a minute into a steady stream), and again when a young account's votes reach full weight. Maintenance:

```sh
cargo run -p rankings-admin -- recompute             # recompute tracker_scores now
cargo run -p rankings-admin -- void-voter <voter-id> # delete one voter's votes, then recompute
```

Browse via `localhost`, not `127.0.0.1`: the session cookie is always `Secure`, and browsers accept that over plain HTTP only on `localhost`.

Sign-in needs a Discord and/or Reddit OAuth app. Register `${PUBLIC_BASE_URL}/auth/discord/callback` (Discord, scope `identify`) or `${PUBLIC_BASE_URL}/auth/reddit/callback` (Reddit "web app", scope `identity`), then fill the matching variables in `.env`. A provider with no credentials is hidden from `/login`.

## Deploy

CI publishes `ghcr.io/kilooscarsix/tracker-rankings` on every push to `main` (`latest`, `main`, `sha-<short>`) and on `v*` tags (semver tags). The image holds `web`, its asset bundle, `rankings-admin` and the seed.

```sh
cp compose.example.yaml compose.yaml   # then set POSTGRES_PASSWORD and COOKIE_KEY in .env
docker compose up -d                   # applies migrations, then starts the site on 127.0.0.1:3000
docker compose run --rm admin import
```

Put a TLS reverse proxy in front, set `PUBLIC_BASE_URL` to its origin and `TRUSTED_PROXIES` to the compose network's gateway. To build the image locally: `docker build -t ghcr.io/kilooscarsix/tracker-rankings:latest .`

To re-vendor the seed, follow the command in `data/trackerpathways/SOURCE` and update its commit, date, checksum and counts.

Checks: `bacon` (clippy by default), or see the full suite in `CLAUDE.md`.

## AI acknowledgement

AI assisted with:

- Writing tests
- Breaking down features into digestible tasks
- General research
- Frontend design

All main line code was written by a human and reviewed.

## Attribution

The tracker and route data is © the trackerpathways contributors, under the MIT License. See `data/trackerpathways/LICENSE`.
