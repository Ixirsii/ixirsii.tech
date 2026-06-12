# AGENTS.md — ixirsii.tech

## Build & run

| Command | Purpose |
|---|---|
| `cargo run` | Build & run the dev server on `127.0.0.1:3000` |
| `cargo build` | Compile only |
| `cargo clippy` | Lint with strict clippy config |
| `cargo fmt` | Format with `edition = "2024"` rules |
| `cargo run --bin seed_admin <username> <password>` | Create/update an admin user (upserts into `admin_users`) |

`build.rs` auto-downloads the standalone Tailwind CSS CLI (via `curl`) and runs `tailwindcss -i src/style/main.css -o src/static/main.css --minify` before every Cargo build. No Node.js or `bun` needed. Tailwind v4 is configured entirely in `src/style/main.css` (no JS config file).

No tests exist (no `tests/` directory, no test modules). Do not add `#[cfg(test)]` without confirming.

## Architecture

- **Backend**: Rust + Axum + Askama templates + SQLx (MySQL). Edition 2024.
- **Frontend**: HTMX (served from `src/static/htmx.min.js`), Tailwind CSS v4, Catppuccin themes via CSS custom properties.
- **Templates**: Askama `*.html` in `templates/`, compiled at build time. Uses `{% extends "base.html" %}` / `{% block content %}`.
- **Static files**: `src/static/` served at `/static/` via `tower-http::services::ServeDir`.
- **CSS**: Split across `src/style/` — `main.css` (entrypoint, theme variables, shared styles) with `@import` for `about.css`, `admin.css`, `auth.css`, `blog.css`, `code.css`, `tag.css`.
- **Entrypoint**: `src/main.rs` — sets up Axum router with public routes, admin routes, auth layer, and static file serving.
- **Page handlers**: Under `src/page/` — `about.rs`, `admin.rs` (dashboard + login), `admin_posts.rs`, `admin_tags.rs`, `blog.rs`, `code.rs`, `home.rs`, `tag.rs`. Re-exported from `src/page/mod.rs`.
- **Auth**: Session-based admin auth via `axum-login`. Login at `/login`, logout at `/admin/logout`. Passwords hashed with Argon2 (`argon2` crate). Admin routes gated by `login_required!`.
- **CSRF**: Token-based CSRF protection on all admin form submissions. Token stored in session, verified on POST. `src/csrf.rs`.
- **DB**: MySQL, connected via `dotenvy` env vars `DATABASE_URL` and `DATABASE_MAX_CONNECTIONS` (defaults to 5). Pool (`Arc<MySqlPool>`) is passed as `AppState` to all handlers. SQLx migrations in `migrations/` run at startup.
- **Posts**: Markdown content in MySQL, rendered server-side via `pulldown-cmark` + `ammonia` (HTML sanitization). Home and blog index use `PAGE_SIZE` (8) for pagination with `div_ceil`.
- **Tags**: Tag cloud with size/color weighting, live search via HTMX (`/tags/search`). Tags are linked to posts via `post_tags` join table.
- **Admin**: Full CRUD for posts and tags. Markdown preview via HTMX. Post filtering/sorting (published, scheduled, unscheduled). Slug uniqueness validation.
- **Code pages**: Static project data defined in `src/page/code.rs` (not in DB).
- **Themes**: 4 Catppuccin flavors (mocha, latte, frappe, macchiato). The blocking `<head>` script sets `data-theme` on `<html>`, resolving `system` to mocha/latte via `window.matchMedia`. Theme persisted in `localStorage`.
- **Binary**: `src/bin/seed_admin.rs` — CLI tool to create/update admin users. Run via `cargo run --bin seed_admin`.
- **Error handling**: Custom `Error` enum in `src/types.rs` with `Result<T>` alias. `dotenvy::Error`, `std::io::Error`, `password_hash::Error`, `MigrateError` all convert into it.
- **Observability**: `src/observability.rs` — request timing middleware via `trace_request()` and tracing init via `init_tracing()`. Console logging always active; conditional OTLP export to Datadog when `OTEL_EXPORTER_OTLP_ENDPOINT` is set. All 29 database functions instrumented with `#[instrument(skip(executor/pool, body, preview, tag_ids))]` for span timing. Datadog APM compatible via `opentelemetry-otlp` HTTP/protobuf exporter.
- **Migrations**: 8 SQLx migration files creating posts, tags, post_tags, admin_users tables with indexes and seed data.

## Notable quirks

- `dotenv().ok()` loads `.env` manually on startup (no auto-load middleware). Missing vars surface as type errors at the first `var()` call.
- Server address is hardcoded to `127.0.0.1:3000` — not configurable via env.
- `build.rs` first reuses a cached standalone Tailwind CLI in `.tailwindcss/`, otherwise falls back to `tailwindcss` on `PATH`, and only then downloads the platform-specific binary via `curl`. It compiles `src/style/main.css` → `src/static/main.css` with `--minify` and declares `cargo:rerun-if-changed=src/style/main.css` (only the root file; imported sub-files don't auto-trigger).
- Askama `#[derive(Template)]` structs are split across `src/page/about.rs`, `admin.rs`, `admin_posts.rs`, `admin_tags.rs`, `blog.rs`, `code.rs`, `home.rs`, and `tag.rs`.
- Shared page helpers live in `src/page/mod.rs`, including `PostView`/`TagView` models, `render_markdown()`, `redirect_target()`, and `slugify()`.
- Strict clippy config in `Cargo.toml` `[lints.clippy]`: `pedantic`/`nursery`/`cargo` = warn, `unwrap_used`/`expect_used`/`missing_errors_doc`/`missing_panics_doc` = deny. `src/database/mod.rs` locally allows `missing_errors_doc`.
- All 26 Catppuccin flavor colors defined in `src/style/main.css` as `--ctp-*` custom properties. No `@catppuccin/tailwindcss` npm dependency.
- CSRF token is stored in the session and verified on every admin POST. Uses a simple constant-time comparison (no per-form tokens). The seed binary is the only way to create admin users — there's no registration UI.
- `README.md` includes a Codeberg Actions CI badge, but no workflow files are checked into the repo yet.
- No pre-commit hooks.
- `rust-toolchain.toml` at repo root pins the Rust version for Railway's Nixpacks builder. Update the `channel` when Rust is upgraded locally.

## If stuck

If the build fails, check `build.rs` first — it may need `curl` and network access the first time if neither `.tailwindcss/` nor a system `tailwindcss` binary is available. If CSS isn't regenerated, check `src/style/main.css` and its `@import`-ed files for syntax errors. Check `src/main.rs` for the route table and `migrations/` if startup fails during `sqlx::migrate!()`. If the app won't start, ensure `DATABASE_URL` is set and the database is reachable. If login fails, re-run `cargo run --bin seed_admin` to ensure the admin user exists with the expected password hash.
