# ixirsii.tech

[![Discord](https://img.shields.io/discord/452233740408717313?color=738ADB&label=discord&logo=discord)][1]
[![License](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)][2]
[![Build](https://codeberg.org/Ixirsii/ixirsii.tech/badges/workflows/build.yml/badge.svg?branch=main&event=push&label=build)][3]

[Personal website][5].

## Tech stack

| Requirement     | Dependency           |
|-----------------|----------------------|
| Language        | [Rust][6]            |
| Database        | PostgreSQL + [SQLx][7] |
| HTTP handler    | [Axum][8]            |
| HTML templates  | [Askama][9]          |
| Markdown parser | [pulldown_cmark][10] |
| Javascript      | [HTMX][11]           |

## Local development

Install Rust 1.95.0, Tailwind CSS CLI 4.3.3 (`tailwindcss` on `PATH`), and
SQLx CLI 0.9.0 (`cargo install sqlx-cli --version 0.9.0 --locked`). With Docker
or OrbStack running:

```sh
docker compose up -d --wait postgres
export DATABASE_URL='postgres://ixirsii:ixirsii_dev_only@127.0.0.1:55432/ixirsii_dev'
cargo sqlx migrate run
docker compose run --rm seed
cargo run
```

Open <http://localhost:3000>. The development admin login is `developer` /
`local-development-only`. Additional local settings are shown in `.env.example`;
the application reads `.env`, but the exported URL above takes precedence.

Compose runs PostgreSQL 18 on loopback port 55432 with a named volume. The explicit
`seed` service always connects to that Compose database, ignores `DATABASE_URL`,
and checks the database and role before inserting fixtures. Reruns add missing
rows without overwriting edits. Fixtures include twelve published posts across
two pages, tied publication dates, tagged and untagged posts, a draft, a scheduled
post, and unused tags. They live in `dev/seed.sql`, outside the migrations and
application startup.

All stored timestamps use PostgreSQL `TIMESTAMPTZ` and Rust `DateTime<Utc>`.
Admin publication inputs and displayed dates use UTC. Usernames, tag names,
and slugs use PostgreSQL's case-sensitive comparisons and uniqueness.

## Database queries and tests

SQLx macros validate queries against PostgreSQL when preparing committed `.sqlx/`
metadata. Ordinary builds use that metadata without a database connection:
`.cargo/config.toml` defaults `SQLX_OFFLINE=true`. SQLx enables only Chrono
for date/time query types; session expiry values are converted before binding.

After changing queries or migrations, run these against the local database above
and commit the updated metadata with the code:

```sh
cargo sqlx migrate run
SQLX_OFFLINE=false cargo sqlx prepare -- --all-targets
SQLX_OFFLINE=false cargo sqlx prepare --check -- --all-targets
cargo fmt --check
cargo clippy --locked --all-targets
cargo test --locked --all-targets
```

Tests include colocated `#[sqlx::test]` cases for real PostgreSQL pagination,
filtering, CRUD, and session persistence/expiry. They require `DATABASE_URL` at
runtime and a role allowed to create databases; the Compose role provides that.
SQLx creates a separate migrated database for each database test. Tests do not
depend on the development fixtures. Offline compilation does not remove this
runtime database requirement.

## Production database

These migrations create a fresh PostgreSQL schema without content or admin users.
Startup applies them automatically. Provision a new PostgreSQL database and switch
the deployment's `DATABASE_URL` when deploying this version. These migrations do
not upgrade or import a MySQL database; an existing content transfer and production
admin provisioning must be handled separately. The old MySQL migrations and
`seed_initial_content` are intentionally removed. Never use the development
credentials or fixture command for production.

## CI

Pull requests and pushes to `main` run the security scans before the Rust build.
The build workflow calls `security.yml` and waits for all checks to succeed before
installing Tailwind or running Cargo. A failed scan or missing Socket API key blocks
the build; fork PRs cannot build because Forgejo withholds that key.

Release tags validate the release, run the same security and build sequence, and
then deploy to Railway. Scans and builds check out the same triggering commit.
The `ci-rust` FreeBSD runner installs Tailwind and builds with `SQLX_OFFLINE=true`;
Railway also uses an offline release build. CI currently does not run the database
tests or verify metadata against a running PostgreSQL schema, so run the checks
above before submitting query changes.

## Todo

- Code review (un-vibe the code)
  - Clean up templates
  - Clean up styles
  - Clean up database
  - Clean up pages
  - Clean up CSRF
  - Clean up observability

[1]: https://discord.gg/KTgshSbEUU
[2]: https://opensource.org/license/bsd-3-clause
[3]: https://codeberg.org/Ixirsii/ixirsii.tech/actions/workflows/build.yml
[5]: https://ixirsii.tech
[6]: https://rust-lang.org
[7]: https://docs.rs/sqlx
[8]: https://docs.rs/axum
[9]: https://docs.rs/askama/
[10]: https://docs.rs/pulldown-cmark
[11]: https://htmx.org
