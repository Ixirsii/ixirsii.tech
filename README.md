# Personal blog

[![Discord](https://img.shields.io/discord/452233740408717313?color=b4befe&label=discord&labelColor=1e1e2e&logo=discord&style=flat)](https://discord.gg/KTgshSbEUU)
[![License](https://img.shields.io/badge/License-BSD_3--Clause-89b4fa?labelColor=1e1e2e&logo=bsd&logoColor=white&style=flat)](https://opensource.org/license/bsd-3-clause)
[![Staging](https://git.9dev.tech/Ixirsii/ixirsii.tech/badges/workflows/staging.yml/badge.svg?branch=main&event=push&label=Staging&labelColor=1e1e2e&style=flat&logo=forgejo&logoColor=white)](https://git.9dev.tech/Ixirsii/ixirsii.tech/actions/workflows/staging.yml)
[![Production](https://git.9dev.tech/Ixirsii/ixirsii.tech/badges/workflows/production.yml/badge.svg?branch=main&event=push&label=Production&labelColor=1e1e2e&style=flat&logo=forgejo&logoColor=white)](https://git.9dev.tech/Ixirsii/ixirsii.tech/actions/workflows/production.yml)

## Links

- [Production](https://ixirsii.tech)
- [Beta](https://beta.ixirsii.tech)
- [Release runner setup and image promotion](docs/image-releases.md)

## Tech stack

| Requirement    | Dependency                                |
| -------------- | ----------------------------------------- |
| Database       | [PostgreSQL](https://www.postgresql.org/) |
| SQL client     | [SQLx](https://docs.rs/sqlx)              |
| HTTP handler   | [Axum](https://docs.rs/axum)              |
| HTML templates | [Askama](https://docs.rs/askama/)         |
| Javascript     | [HTMX](https://htmx.org)                  |

To build from source, install [Tailwind CSS CLI 4.3.5](https://github.com/tailwindlabs/tailwindcss/releases/tag/v4.3.5)
as `tailwindcss` on your `PATH`. `cargo build --locked` regenerates `public/main.css`.

## Run locally with PostgreSQL

The [Compose service](compose.yaml) runs PostgreSQL 18 on `127.0.0.1:55432`.
With Rust and Docker Compose installed, run these commands from the repository root:

```sh
cargo install sqlx-cli --version 0.9.0 --locked --no-default-features --features postgres
docker compose up -d --wait postgres
export DATABASE_URL='postgres://ixirsii:ixirsii_dev_only@127.0.0.1:55432/ixirsii_dev'
cargo sqlx migrate run
cargo run --locked
```

Open [localhost:3000](http://localhost:3000). Apply migrations before building
against a fresh database: SQLx checks queries during compilation when `DATABASE_URL`
is set. For sample posts, tags, and a local admin, run `docker compose run --rm seed`
after migrating. [`.env.example`](.env.example) lists the local connection settings.

After an update, run `cargo sqlx migrate run` from the new release before
starting the server. Startup checks the applied migration versions, checksums,
and success flags; it never applies migrations. If the database has newer
migrations, start the matching release instead of retrying the older binary.

To customize the installed site's name, colors, or icons, edit
[`public/site.webmanifest`](public/site.webmanifest) and its two PNGs, then
rebuild. Browsers revalidate the manifest and icons at their stable URLs.

## Writing blog posts

Enter publication times in UTC. Public pages show published and updated dates and times
in the reader’s local timezone, or in UTC when JavaScript is disabled.

Use `##` for sections and `###` for subsections. Individual posts link to these
headings in an **On this page** sidebar, shown above the article on small screens.

Blog posts and admin Markdown previews can display only images served from the
site’s own origin. Use ordinary Markdown links for images hosted elsewhere.

## Refresh the dotfiles project page

`/code/dotfiles` displays the dotfiles README with local screenshots and no
badges. To refresh it, run `scripts/sync-dotfiles-readme ~/git/dotfiles HEAD`
with Python 3.8+ and Git. Replace `HEAD` with a tag or commit to select another
revision. Review and commit the updated snapshot and screenshots before deploying.
The build indexes the screenshots automatically; it does not need the dotfiles checkout.

## Create an admin account

With `DATABASE_URL` set to the intended, migrated PostgreSQL database, run this
from an interactive terminal using the release executable:

```sh
ixirsii-tech admin create <username>
```

Replace `<username>` with the case-sensitive login name. The command prompts
twice without echoing the password and exits after creating the account. An
existing username is an error; its password is never changed. Sign in at `/login`.

From a checkout, use `cargo run -- admin create <username>`. Plain `cargo run`
starts the local server.

Admin login limits are shared by all replicas using the same database and survive
restarts. By default, each client has 20 attempts per rolling five minutes, and
each account/client pair has 5. Successful logins count. Use the same `AUTH_*`
limit and client-identity settings on every replica. When upgrading from
process-local limits, the shared budgets start empty; drain the old replicas
to finish the switch.

## Server logging

The server writes structured JSON logs to stdout. Set `RUST_LOG` to adjust
verbosity (default: `info`). To send logs and HTTP metrics to Grafana Cloud,
set `OTEL_EXPORTER_OTLP_ENDPOINT` to the stack's base OTLP URL and
`OTEL_EXPORTER_OTLP_HEADERS` to the authorization header in the service's
secret variables. Set `DEPLOYMENT_ENVIRONMENT` to `staging` or `production`.
Without an endpoint, logging stays local. Health probes are excluded from
request telemetry.

Browser asset attributions are available at `/licenses`.

## Todo

- Code review (un-vibe the code)
  - Clean up templates
  - Clean up styles
  - Clean up database
  - Clean up pages
  - Clean up CSRF
