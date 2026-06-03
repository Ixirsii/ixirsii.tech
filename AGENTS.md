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
- **Entrypoint**: `src/main.rs` — sets up Axum router (`/`, `/about`, `/blog`, `/blog/{slug}`, `/code`, `/code/{slug}`, `/tags`, `/tags/{slug}`, `/static/*`).
- **Pages**: Route handlers live under `src/page/` (`about.rs`, `blog.rs`, `code.rs`, `home.rs`, `tag.rs`) and are re-exported from `src/page/mod.rs`.
- **DB**: MySQL, connected via `dotenvy` env vars `DATABASE_URL` and `DATABASE_MAX_CONNECTIONS`. Pool is passed as `AppState` to all handlers.
- **Posts**: Markdown posts in the database, rendered server-side via `pulldown-cmark`. Home page lists up to 10 full posts; `/blog/{slug}` serves individual posts.
- **Migrations**: SQLx migrations live under `migrations/` and run automatically at startup via `sqlx::migrate!()`.
- **Repo**: hosted on Codeberg (not GitHub).

## Notable quirks

- `dotenv().ok()` loads `.env` manually on startup (no auto-load middleware). The `.ok()` silently swallows `.env` load failures; missing vars surface cryptically at the first `var()` call instead.
- Server address is hardcoded to `127.0.0.1:3000` — not configurable via env.
- `build.rs` first reuses a cached standalone Tailwind CLI in `.tailwindcss/`, otherwise falls back to `tailwindcss` on `PATH`, and only then downloads the platform-specific binary via `curl`. It recompiles `src/main.css` to `src/static/main.css` with `--minify` and declares `cargo:rerun-if-changed=src/main.css`.
- Askama derives (`#[derive(Template)]`) are split across `src/page/about.rs`, `src/page/blog.rs`, `src/page/code.rs`, `src/page/home.rs`, and `src/page/tag.rs`. Current template structs are `AboutPage`, `BlogIndexPage`, `BlogPostPage`, `CodePage`, `CodeProjectPage`, `HomePage`, `TagsPage`, and `TagPostsPage`.
- Shared page helpers live in `src/page/mod.rs`, including Markdown rendering and the `PostView`/tag view models used by multiple handlers.
- Strict clippy config in `Cargo.toml` `[lints.clippy]`: `pedantic`/`nursery`/`cargo` = warn, `unwrap_used`/`expect_used`/`missing_errors_doc`/`missing_panics_doc` = deny. `rustfmt.toml` sets `edition = "2024"`.
- `src/database/mod.rs` locally allows `clippy::missing_errors_doc`, so database helpers do not all carry individual `# Errors` sections.
- All 26 Catppuccin flavor colors defined directly in `src/main.css` as `--ctp-*` custom properties. No `@catppuccin/tailwindcss` npm dependency. `src/main.css` has `@theme inline { --color-ctp-*: var(--ctp-*) }` for Tailwind registration, `:root` with mocha defaults, and `[data-theme="..."]` blocks for each flavor. The blocking `<head>` script always sets `data-theme` (resolving `system` to mocha/latte via `window.matchMedia`).
- `README.md` currently includes a Codeberg Actions CI badge/link, but no workflow files are checked into the repo yet.
- No pre-commit hooks.

## If stuck

If the build fails, check `build.rs` first — it may need `curl` and network access the first time if neither `.tailwindcss/` nor a system `tailwindcss` binary is available. If CSS isn't regenerated, check `src/main.css` for syntax errors. Check `src/main.rs` for the route table and `migrations/` if startup fails during `sqlx::migrate!()`. If the app won't start, ensure `DATABASE_URL` is set and the database is reachable.
