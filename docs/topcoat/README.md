> Copied from `~/Documents/Viridian/zayden/docs/topcoat/`. API facts (signatures, status codes, observed behaviour) hold for any project on the pinned version. Everything the files say about `web`, "this repository" or "the app" (`Web:` citations, installed state, build and deploy details, "impact on this repo" columns) describes zayden, not this project: use it as a worked example. This project's own Topcoat wiring is in [This repository](#this-repository) below and wins where the two differ.

# Topcoat reference

Verified against: topcoat 0.10.0

Topcoat is pre-1.0 and its API changes between minor releases. This directory records what the pinned version actually provides, so code is written against checked facts instead of memory or search results. Read the relevant file before writing Topcoat code.

| File | Covers |
|---|---|
| [core.md](core.md) | Features, `view!`, components, client expressions, router, request context, forms, cookies, origin policy, assets, runtime script, head and meta, errors, test harness |
| [ui.md](ui.md) | Topcoat UI registry components, the `neutral` theme, Geist font, icons, wiring the installed components into `web` |
| [tooling.md](tooling.md) | `topcoat` CLI, Tailwind build integration, font and icon features, how `web` is built locally, in Docker and in CI |
| [changes-0.9-to-0.10.md](changes-0.9-to-0.10.md) | Release notes and public API differences between 0.9.0 and 0.10.0, with their impact on this repository |

## Status labels

- **VERIFIED**: found in the pinned source and compiled in a probe crate.
- **UNVERIFIED**: documented upstream but not compiled here. Probe it before relying on it.
- **NO EQUIVALENT**: the feature does not exist in the pinned version. The entry names the fallback used here.

## Sources, in order of authority

1. The pinned crate sources (`~/.cargo/registry/src/*/topcoat-*-<version>/`) and docs.rs for that version.
2. Compiler output.
3. Observed runtime behavior.

An API that is not in (1) and has not compiled in (2) is treated as nonexistent.

`Probe:` lines name the local verification crates (under `scratch/topcoat-kb/`) the examples were compiled in. They are not committed. To re-verify an entry, paste its example into a crate that depends on `topcoat = "=<version>"` with the listed features and build it.

## Updating for a new Topcoat release

`web/tests/topcoat_reference.rs` fails when the `topcoat` version in `Cargo.lock` differs from the `Verified against` line above. When bumping Topcoat:

1. Read the release notes and diff the public API of the old and new crate sources.
2. Add `changes-<old>-to-<new>.md` in the format of the existing one, with the impact on this repository.
3. Re-check every entry in `core.md`, `ui.md` and `tooling.md` that the diff touches, recompile its example, and update its source citation.
4. Update the `Verified against` line in every file, then run `cargo test -p web --test topcoat_reference`.

## This repository

Checked against the files on 2026-10-21. Update this section when the wiring changes.

| Area | State | Files |
|---|---|---|
| Features | runtime: `asset`, `compression`, `cookie`, `icon-iconify`, `router`, `runtime`, `serve`, `session`, `view`; build: `icon-iconify`, `tailwind`, `ui` | `web/Cargo.toml` |
| Build script | Tailwind 4.3.2 from `web/styles.css` (checksum on Linux x86-64, `TAILWIND_CLI` override); Iconify set staged from the committed `web/icons/lucide.json` (`cache_dir("icons")`, ui.md "Icons" option B), so the build fetches no icons | `web/build.rs` |
| Stylesheet | `web/styles.css` is the `neutral` theme and the only Tailwind input: `@import "tailwindcss" source(none)`, `@source "./src/**/*.rs"`, then one `@source not` line per unlinked component | `web/styles.css` |
| Components | All 31 registry components are on disk; `web/src/components.rs` (no `mod.rs`) declares the linked ones under one module-level `#![expect(unreachable_pub, ...)]`. Linked: `badge`, `button`, `label`, `radio_group`, `table`, plus the app's own `score`. `select.rs` is the registry file; there is no app `select` | `web/src/components.rs`, `web/components.toml` |
| Sessions | `topcoat::session` (core.md section 9): `__Host-session` cookie, `sessions.token_hash` stores the `TokenHash` | `web/src/auth/sessions.rs`, `web/src/router.rs` |
| Forms | `Form<T>` with `serde::Deserialize` structs; POST routes answer `see_other` | `web/src/votes.rs` |
| Layers | Path-scoped `LayerFn` rate limits on `/auth`, `/logout`, `/vote`, `/me/delete` | `web/src/rate_limit.rs` |
| Tests | Router tests with a stand-in asset bundle (core.md section 16.2) | `web/tests/common/mod.rs` |

Gotchas found here:

- `class!` concatenates classes and does not merge Tailwind conflicts. A component that sets a base layout class (`radio_group` sets `grid gap-3`) keeps it when you pass `class="flex ..."`, and the stylesheet order decides which wins (`grid` did). Pass classes that compose with the base instead: `grid-flow-col auto-cols-max gap-x-4` for a horizontal `radio_group` (`web/src/pages/tracker.rs`).
- A build without `DATABASE_URL` in the environment lets the `sqlx` macros read `.env`; use `SQLX_OFFLINE=true` for `topcoat asset bundle -p web` when `.env` points at a database you cannot reach.

## Updating vendored UI components

The files in `web/src/components/` that `web/components.toml` lists come from `topcoat-ui-registry`. Every installed file carries lint fix-ups and `topcoat fmt` formatting (ui.md "Using it in this app", step 3), so `topcoat ui add <name> --overwrite` would discard them.

1. Diff each file against the registry source of the new version, `~/.cargo/registry/src/*/topcoat-ui-registry-<version>/src/components/<name>.rs`, and note the local edits.
2. Run `topcoat ui add <name> --overwrite`, then re-apply the local edits.

## Wiring an unlinked component

1. Add `pub mod <name>;` to `web/src/components.rs`, plus any registry siblings it imports through `super::` (ui.md "Component index").
2. Delete its `@source not "./src/components/<name>.rs";` line from `web/styles.css`.
3. Keep its `iconify_icon!(...)` calls: they read the vendored `web/icons/lucide.json`, a trimmed set holding the seven icons the registry uses. An icon outside it is a compile error; add its body to the file (ui.md "Icons", option B).
4. Fix any lints the newly compiled file raises without silencing them (ui.md "Using it in this app", step 3).
