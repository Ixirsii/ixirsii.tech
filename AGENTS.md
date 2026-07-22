# AGENTS.md - ixirsii.tech

## Prerequisites

- Rust 1.95.0, pinned by `rust-toolchain.toml`.
- MySQL reachable through `DATABASE_URL`.
- Tailwind CSS CLI on `PATH` as `tailwindcss`. Railway installs Tailwind 4.3.3 through `railpack.json`; local and CI environments must provide it separately.

There is no Node.js project or JavaScript package-manager setup.

## Build and run

| Command | Purpose |
|---|---|
| `cargo run` | Build and run on `0.0.0.0:${PORT:-3000}` |
| `cargo build` | Compile the application |
| `cargo check` | Type-check the application |
| `cargo clippy` | Lint with the strict configuration in `Cargo.toml` |
| `cargo fmt` | Format using Rust 2024 edition rules |

Cargo build, check, run, and Clippy commands execute `build.rs`. It runs:

```text
tailwindcss -i src/style/main.css -o src/static/main.css --minify
```

The build script also fingerprints every file under `src/static/` with SHA-256 and generates `OUT_DIR/static_assets.rs`. It watches every top-level `.css` file in `src/style/` and every static asset. A missing `tailwindcss` executable causes these Cargo commands to fail during the build script.

No tests currently exist: there is no `tests/` directory or Rust test module. Do not introduce a test structure without confirming the intended approach.

The repository currently has no admin-user creation binary or registration flow. `admin_users` must be provisioned outside the application until a supported provisioning command is added.

## Runtime configuration

`dotenv().ok()` attempts to load a local `.env` before startup. The application uses these variables:

| Variable | Required | Default | Purpose |
|---|---:|---|---|
| `DATABASE_URL` | Yes | None | MySQL connection URL |
| `DATABASE_MAX_CONNECTIONS` | No | `5` | SQLx pool size; invalid values also fall back to 5 |
| `PORT` | No | `3000` | Listen port; the host is fixed to `0.0.0.0` |
| `CSRF_ALLOWED_ORIGINS` | No | Origin check disabled | Comma-separated allowed origins for admin state-changing requests |
| `RUST_LOG` | No | `info` | Tracing filter |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | No | Disabled | OTLP HTTP/protobuf trace endpoint |

Startup connects to MySQL and runs embedded SQLx migrations before binding the listener. The `/health` endpoint is only available after startup succeeds and returns `200 OK` without querying the database.

## Architecture

- **Backend**: Rust 2024, Axum 0.8, Askama, SQLx, and MySQL.
- **Frontend**: Askama templates, vendored HTMX 2.0.10, Tailwind CSS v4, and Catppuccin CSS custom properties.
- **Entrypoint and routing**: `src/main.rs` initializes tracing and the database, runs migrations, builds public and protected routers, configures sessions, and serves embedded assets.
- **Application state**: `AppState` in `src/types.rs` contains `Arc<MySqlPool>` and configured CSRF origins. It also implements the `axum-login` authentication backend.
- **Page handlers**: `src/page/` contains `about.rs`, `admin.rs`, `admin_posts.rs`, `admin_tags.rs`, `blog.rs`, `code.rs`, `error.rs`, `hall_of_fame.rs`, `home.rs`, and `tag.rs`. They are re-exported by `src/page/mod.rs`.
- **Templates**: Askama templates live in `templates/`. Full pages extend `base.html`; `tags_cloud.html` and `admin_markdown_preview.html` are HTMX fragments.
- **Database**: `src/database/` is split into posts, tags, post-tag joins, transactions, users, and shared types. All 29 public database operations use tracing instrumentation.
- **Migrations**: Eight SQLx migrations create posts, tags, post-tag joins, the publication index, and admin users, and seed post/tag demonstration data. Migrations run at process startup and do not seed an admin user.
- **Errors**: `src/types.rs` defines the internal `Error` and `Result<T>` types. `src/page/error.rs` renders styled public 404 and 500 responses.
- **Observability**: `src/observability.rs` always enables console tracing. It adds an OTLP HTTP/protobuf exporter when `OTEL_EXPORTER_OTLP_ENDPOINT` is set. Request spans record method, raw request path, and response status.

## Routes and pages

Public routes include Home, About, Blog, Code projects, Hall of Fame, Tags, login, and health. The fallback renders a styled 404 page.

- `PAGE_SIZE` is 8. Home shows the first eight published posts with full rendered bodies and no pagination.
- The blog index shows eight previews per page with Previous/Next navigation. Blog detail pages render complete posts.
- Markdown uses default `pulldown-cmark` parsing followed by `ammonia` sanitization.
- The tag index uses HTMX search at `/tags/search`; tag sizes and colors are recalculated from the filtered result.
- Code project data is hard-coded in `src/page/code.rs`, not stored in MySQL.
- HTMX is used only for tag filtering and admin Markdown previews.

The protected `/admin` router provides post and tag CRUD, Markdown previews, and logout. It is gated by `login_required!` and the CSRF Origin/Referer middleware. The admin post interface exposes status filtering and sorting, but the current database query prefilters published posts before applying status filters, so scheduled and unscheduled filters do not return their intended rows. The admin post list also shows only the first eight rows and has no pagination.

## Authentication and CSRF

- Login is at `/login`; logout is a POST to `/admin/logout`.
- Password verification uses Argon2 hashes stored in `admin_users`.
- Sessions use `tower-sessions` `MemoryStore`, `SameSite=Lax`, and secure cookies outside debug builds. Sessions do not survive restarts or work across multiple application replicas.
- Admin POST handlers verify a token stored in the session. Tokens rotate after successful login and successful post/tag mutations.
- Token verification uses ordinary string equality and does not use per-form tokens.
- Admin state-changing requests additionally check `Origin` or `Referer` against `CSRF_ALLOWED_ORIGINS`. This middleware check is skipped when no origins are configured. Login is outside that middleware and relies on its form token.

## Static assets and CSS

- `src/static/` is embedded into the executable with `include_dir`; it is not served from the filesystem with `ServeDir`.
- Normal assets are available only through build-generated fingerprinted `/static/*` paths and receive a one-year immutable cache header.
- Templates resolve asset paths through `crate::static_assets::static_url()`.
- Embedded `/.well-known/*` files use non-fingerprinted paths and a one-hour cache header.
- Source CSS is split across `src/style/main.css`, `about.css`, `admin.css`, `auth.css`, `blog.css`, `code.css`, and `tag.css`. Tailwind v4 configuration lives in CSS through `@theme`; there is no JavaScript config.
- Four complete Catppuccin flavors are defined: mocha, latte, frappe, and macchiato.
- Explicit theme choices are stored in `localStorage`. Selecting System removes the stored choice and resolves to mocha or latte from `prefers-color-scheme` on page load or selection. There is no live listener for later system-theme changes.

## Linting and CI

`Cargo.toml` denies unsafe code and enables Clippy `cargo`, `nursery`, and `pedantic` warnings. It denies `expect_used`, `unwrap_used`, `missing_errors_doc`, and `missing_panics_doc`.

Forgejo CI is defined in `.forgejo/workflows/ci.yml`. For pushes and pull requests targeting `main`, it runs Cargo check, Trivy, and Socket CLI. Successful `main` runs deploy to Railway. The workflow currently does not run formatting, Clippy, or tests, and its Cargo container does not explicitly install Tailwind.

Railway uses the Railpack builder configured by `railway.toml` and `railpack.json`. It builds with `cargo build --release --locked`, starts `./target/release/ixirsii-tech`, checks `/health`, and restarts on failure.

There are no pre-commit hooks.

## If stuck

- If any Cargo command fails while running `build.rs`, verify that a compatible `tailwindcss` executable is on `PATH`, then inspect `src/style/main.css` and its imports.
- If CSS or an asset URL is stale, inspect `build.rs`, the generated `src/static/main.css`, and the static manifest generation inputs.
- If startup fails before listening, verify `DATABASE_URL`, MySQL connectivity, and the migrations in `migrations/`.
- If admin state-changing requests return `403`, verify the session token and `CSRF_ALLOWED_ORIGINS` values, including scheme and port.
- If login fails, verify that `admin_users` contains the expected username and a valid Argon2 password hash. There is no in-repository seeding command.
- Check `src/main.rs` for the authoritative route and middleware layout.
