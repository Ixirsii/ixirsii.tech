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

| Requirement    | Dependency                        |
| -------------- | --------------------------------- |
| Database       | [SQLx](https://docs.rs/sqlx)      |
| HTTP handler   | [Axum](https://docs.rs/axum)      |
| HTML templates | [Askama](https://docs.rs/askama/) |
| Javascript     | [HTMX](https://htmx.org)          |

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
revision. Review and commit the updated snapshot, screenshots, and asset catalog
before deploying. Builds do not need the dotfiles checkout.

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
