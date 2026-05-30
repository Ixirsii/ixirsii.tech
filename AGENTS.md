# AGENTS.md — ixirsii.tech

## Build & run

| Command | Purpose |
|---|---|
| `cargo run` | Build & run the dev server on `127.0.0.1:3000` |
| `cargo build` | Compile only |
| `cargo clippy` | Lint with strict clippy config (denies `unwrap_used`, `expect_used`, `missing_errors_doc`, `missing_panics_doc`) |
| `cargo fmt` | Format with `edition = "2024"` rules |

`build.rs` auto-downloads the standalone Tailwind CSS CLI (via `curl`) and runs `tailwindcss -i src/main.css -o src/static/main.css --minify` before every Cargo build. No Node.js or `bun` needed. Tailwind v4 is configured entirely in `src/main.css` (no JS config file).

No tests exist (no `tests/` directory, no test modules). Do not add `#[cfg(test)]` without confirming.

## Architecture

- **Backend**: Rust + Axum + Askama templates + SQLx (MySQL). Edition 2024.
- **Frontend**: HTMX (served from `src/static/htmx.min.js`), Tailwind CSS v4, Catppuccin Mocha theme via CSS custom properties.
- **Templates**: Askama `*.html` in `templates/`, compiled at build time. Uses `{% extends "base.html" %}` / `{% block content %}`.
- **Static files**: `src/static/` served at `/static/` via `tower-http::services::ServeDir`.
- **Entrypoint**: `src/main.rs` — sets up Axum router (`/`, `/about`, `/blog`, `/blog/{slug}`, `/code`, `/static/*`).
- **DB**: MySQL, connected via `dotenvy` env vars `DATABASE_URL` and `DATABASE_MAX_CONNECTIONS`. Pool is passed as `AppState` to all handlers.
- **Posts**: Markdown posts in the database, rendered server-side via `pulldown-cmark`. Home page lists up to 10 full posts; `/blog/{slug}` serves individual posts.
- **Repo**: hosted on Codeberg (not GitHub).

## Notable quirks

- `dotenv().ok()` loads `.env` manually on startup (no auto-load middleware). The `.ok()` silently swallows `.env` load failures; missing vars surface cryptically at the first `var()` call instead.
- Server address is hardcoded to `127.0.0.1:3000` — not configurable via env.
- `build.rs` auto-downloads the platform-specific standalone Tailwind CSS CLI binary (cached in `.tailwindcss/`) and runs it. Aborts the build with stderr output on failure. Requires `curl` to be installed.
- Askama derives (`#[derive(Template)]`) are in `src/handler/page.rs`. Structs are ordered alphabetically (AboutPage, BlogIndexPage, BlogPostPage, CodePage, HomePage), then private helpers.
- Handlers are re-exported via `pub use crate::handler::page::*;` in `src/handler/mod.rs` (before the `mod page` declaration).
- Strict clippy config in `Cargo.toml` `[lints.clippy]`: `pedantic`/`nursery`/`cargo` = warn, `unwrap_used`/`expect_used`/`missing_errors_doc`/`missing_panics_doc` = deny. `rustfmt.toml` sets `edition = "2024"`.
- All 26 Catppuccin flavor colors defined directly in `src/main.css` as `--ctp-*` custom properties. No `@catppuccin/tailwindcss` npm dependency. `src/main.css` has `@theme inline { --color-ctp-*: var(--ctp-*) }` for Tailwind registration, `:root` with mocha defaults, and `[data-theme="..."]` blocks for each flavor. The blocking `<head>` script always sets `data-theme` (resolving `system` to mocha/latte via `window.matchMedia`).
- No CI, no pre-commit hooks.

## If stuck

If the build fails, check `build.rs` first — it requires `curl` to be installed and network access for the initial Tailwind CLI download. If CSS isn't regenerated, check `src/main.css` for syntax errors. Check `src/main.rs` for the route table. If the app won't start, ensure `DATABASE_URL` is set (the app fails hard if unreachable).
