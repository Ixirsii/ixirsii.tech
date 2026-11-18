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
cargo clippy --locked --all-targets -- -D warnings
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

## Login identity and throttling

Set `AUTH_CLIENT_IP_MODE` explicitly in production:

| Mode | Client identity |
| --- | --- |
| `direct` | TCP peer; all forwarding headers are ignored. |
| `trusted-proxy` | Nearest untrusted address walking `X-Forwarded-For` right to left, only when the TCP peer belongs to `AUTH_TRUSTED_PROXY_CIDRS`. |
| `railway` | Exactly one valid `X-Real-IP` supplied by Railway's public HTTP edge. |

`railway.toml` selects `railway` at startup. Railway's edge replaces caller-supplied
`X-Real-IP`; the application does not read `CF-Connecting-IP` directly or guess
Railway proxy CIDRs. This deployment assumes all login traffic traverses the
public HTTP edge. Private-network and TCP-proxy paths bypass edge sanitization
and must not be exposed to untrusted callers when using this mode.
See [Railway's networking specifications](https://docs.railway.com/networking/public-networking/specs-and-limits)
and [the forwarding-header clarification](https://station.railway.com/questions/need-authoritative-client-ip-proxy-tru-090eac86).

Trusted-proxy mode requires nonempty CIDRs and rejects universal `/0` trust.
Missing, malformed, oversized, or entirely trusted forwarding chains are rejected
for trusted peers. Railway mode rejects missing, malformed, or duplicate
`X-Real-IP` headers. Neither falls back to a shared proxy address.
An unset mode preserves direct behavior (or trusted-proxy behavior when CIDRs
are supplied), with a warning in release builds. Startup logs the selected mode;
rejection logs distinguish invalid identity, client limits, account/client limits,
and exhausted password-verification capacity without logging credentials.

The default budgets are five attempts per **account + client** and twenty attempts
per **client**, each in a five-minute sliding window. Known and unknown accounts
use the same policy. Attempts reserve capacity before password verification,
including successful attempts. Rejected attempts do not extend the window.
An attacker on another IP cannot consume the administrator's account/client
budget. Someone sharing the administrator's actual public IP still shares the
client budget; recovery is to wait for admitted attempts to expire or use another
network. Changing the username does not bypass the overall client budget.

This deliberately trades a global account lockout for greater resistance to
targeted denial of service: distributed attackers get a separate budget per IP.
Use strong administrator passwords. The Argon2 concurrency cap still bounds
password-verification work. Limits and bounded identity maps are process-local;
replicas have independent budgets, restarts reset them, and a full map rejects
new identities until entries expire.

### Deployment verification

Before closing IXI-4, verify the deployed configuration using controlled clients
and correlate Railway request logs with application rejection logs:

1. Through Cloudflare, confirm two different public client IPs get independent
   budgets despite sharing the same Railway transport proxy.
2. Repeat over IPv4 and IPv6. Railway's Cloudflare client-IP derivation is confirmed
   for IPv4 origin connections; do not assume the IPv6 origin path behaves identically.
3. Through both the custom domain and the generated Railway domain, submit forged
   `X-Real-IP`, `X-Forwarded-For`, and `CF-Connecting-IP`. Confirm the edge-generated
   client identity cannot be changed by these headers.
4. Using a controlled account, exhaust one account/client budget and confirm valid
   login from another client, then recovery on the original client after expiry.
5. Confirm there is no untrusted private-network or TCP-proxy path to login.

## CI

Pull requests and pushes to `main` run the build workflow. Pushes to `main` also
run Gitleaks, Semgrep, and Trivy in the independent security workflow.

Pushes to `main` also run `deploy.yml`, which requires both security and build
checks before deploying to Railway staging. Release tags run `release.yml`,
validate the release and require a successful staging deployment of the exact
commit, then run both checks before deploying to production. Scans and builds
check out the same triggering commit.

Both environments use the reusable `railway.yml` workflow and wait for that
deployment's successful health-check result. Configure a Railway `staging`
environment with its own database and runtime variables, and set the Forgejo
`RAILWAY_STAGING_TOKEN` secret to its environment-scoped token. Production keeps
the `RAILWAY_TOKEN` secret. Both use `RAILWAY_SERVICE`, which must identify the
service in each environment (use the shared service name). Deployment gates and
health-check waits are tested with Node.js in the build workflow.
The `ci-rust` FreeBSD runner installs Tailwind and PostgreSQL 18, checks formatting,
runs Clippy with warnings denied, runs all tests with the locked dependency graph,
and builds with `SQLX_OFFLINE=true`. Tests use a disposable PostgreSQL cluster
under `/tmp` running as the `postgres` package user, with only a private Unix
socket and no TCP listener. The cluster is stopped and removed on exit. Railway
also uses an offline release build. CI does not refresh or verify SQLx metadata
online, so run the metadata checks above before submitting query changes.

## Content and browser behavior

Blog and tag indexes show eight posts per page, ordered by publication date and
ID. Invalid, zero, overflowing, or out-of-range page numbers return 404; page one
of an empty index remains valid. Article sidebars show metadata and up to four
published posts ranked by shared tags, then publication date and ID.

The RSS 2.0 feed is at `/feed.xml`, is linked for discovery in every page, and
contains the newest 20 published posts. Item IDs survive slug changes. Links use
the canonical `https://ixirsii.tech` origin; the feed uses previews when available
and sanitized Markdown bodies otherwise. Responses are cached for five minutes.
The stable `/site.webmanifest` URL revalidates on each use and references immutable,
fingerprinted icons.

Admin titles/slugs allow 255 characters, previews 512 characters, and tag names/slugs
127 characters, matching PostgreSQL character limits. Slugs use lowercase ASCII
letters and numbers separated by single hyphens: `[a-z0-9]+(-[a-z0-9]+)*`.
Post bodies have an application limit of 65,535 UTF-8 bytes. Server validation is
authoritative; database uniqueness conflicts return field-level 409 responses.

## Login challenges and browser security

Anonymous login pages use a signed, HttpOnly, SameSite=Strict cookie instead of
creating rows in `tower_sessions`. The cookie and submitted form challenge must
match, and the server verifies the signature and a 30-minute expiry. Release
builds use a Secure `__Host-login-csrf` cookie with Path=/ and no Domain. The
challenge confers no authentication; successful login still rotates the session
ID and creates a new session CSRF token, then clears the login challenge cookie.
Failed login attempts remain subject to the existing rate limits.

Migration `20260923000003` adds a singleton signing-key table. Startup generates
its 64-byte random key once; all replicas load that same key, so login challenges
survive restarts and requests can move between replicas. No new environment
variable is required. Treat this table as secret data alongside the session store;
rotate its key with a coordinated restart to invalidate outstanding login forms.
This follows the [signed double-submit cookie approach](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html#signed-double-submit-cookie-recommended).

All routes set `Content-Security-Policy: frame-ancestors 'none'`,
`X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, and
`Referrer-Policy: strict-origin-when-cross-origin`. The framing policy allows the
existing scripts, styles, and HTMX requests. Login/admin responses remain private
and uncacheable.

## Todo

- Code review (un-vibe the code)
  - Clean up templates
  - Clean up styles
  - Clean up database
  - Clean up pages
  - Clean up CSRF
- Re-add observability with Grafana Cloud

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
