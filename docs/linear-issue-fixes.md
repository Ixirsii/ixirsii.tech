# Linear issue pass — 2026-09-23

Reviewed all 20 open issues in the ixirsii.tech project, including their full descriptions and comments. Implemented 16, confirmed two were already covered by the existing checkout, and deferred two with Ryan's agreement. Implementation is on `codex/linear-issue-fixes`; deployment is separate.

## Implemented

| Issue | Result |
|---|---|
| [IXI-1](https://linear.app/ixirsii/issue/IXI-1) | Centralized framing protection, `nosniff`, and referrer policy on public, login, admin, error, and static responses. |
| [IXI-2](https://linear.app/ixirsii/issue/IXI-2) | Canonical slug validation and PostgreSQL character limits; a 65,535-byte Markdown limit; form feedback and matching HTML constraints. Tag slug `search` is reserved for the existing search route. |
| [IXI-3](https://linear.app/ixirsii/issue/IXI-3) | Reusable release/PR build runs formatting, strict Clippy, locked PostgreSQL tests, then the build. Disposable PostgreSQL uses a private Unix socket. |
| [IXI-5](https://linear.app/ixirsii/issue/IXI-5) | Signed, expiring login challenge cookies eliminate anonymous session-row creation. A singleton database key supports restarts and multiple replicas. Successful login still rotates the session ID and session CSRF token. |
| [IXI-7](https://linear.app/ixirsii/issue/IXI-7) | Blog page validation returns 404 for malformed, zero, overflowing, or unavailable pages. Existing deterministic ordering and wide offset arithmetic are retained. |
| [IXI-8](https://linear.app/ixirsii/issue/IXI-8) | Eight-post tag pagination, matching published counts, stable ordering before joining tags, and preview-only body projection. |
| [IXI-10](https://linear.app/ixirsii/issue/IXI-10) | Linked manifest has site names, install metadata, and working fingerprinted icons. The manifest revalidates; icons stay immutable. |
| [IXI-11](https://linear.app/ixirsii/issue/IXI-11) | Theme buttons support keyboard activation, selected state, Escape, and focus restoration. Header icon links have names and larger targets. |
| [IXI-12](https://linear.app/ixirsii/issue/IXI-12) | PostgreSQL unique-constraint races return field-level 409 responses for post slugs and tag names/slugs. Other failures retain 500 behavior and database logging. |
| [IXI-13](https://linear.app/ixirsii/issue/IXI-13) | Light theme is labeled “Yulia (Light)” while keeping the existing Latte palette and stored theme value. |
| [IXI-15](https://linear.app/ixirsii/issue/IXI-15) | Article sidebar contains metadata and up to four published related posts, ranked by shared tags, date, and ID. It stacks below the article on mobile. |
| [IXI-16](https://linear.app/ixirsii/issue/IXI-16) | Removed the unused loading component. |
| [IXI-17](https://linear.app/ixirsii/issue/IXI-17) | Centered, constrained login form with consistent spacing and visible keyboard focus. |
| [IXI-18](https://linear.app/ixirsii/issue/IXI-18) | Discoverable RSS 2.0 feed at `/feed.xml`, with 20 published posts, canonical links, stable item IDs, XML escaping, and five-minute caching. |
| [IXI-19](https://linear.app/ixirsii/issue/IXI-19) | Responsive header grid, wrapping navigation/taglines, consistent gutters, and usable controls at narrow widths. |
| [IXI-33](https://linear.app/ixirsii/issue/IXI-33) | Completed the CSS audit's reading columns, Markdown lists, focus treatment, action sizing, destructive buttons, shared names, responsive grids, semantic tag colors, and reduced-motion behavior. |

## Already implemented in the starting checkout

- [IXI-6](https://linear.app/ixirsii/issue/IXI-6): UTC editor input/output, `DateTime<Utc>`, `TIMESTAMPTZ`, and publication predicates were implemented by the PostgreSQL migration. Existing UTC round-trip tests pass.
- [IXI-9](https://linear.app/ixirsii/issue/IXI-9): `build.rs` watches style, template, migration, and static directories recursively through Cargo's directory change tracking. Static coverage and fingerprint tests pass. The explicit asset table still requires an entry when adding an asset, as documented in AGENTS.md.

## Deferred with Ryan's agreement

- [IXI-14](https://linear.app/ixirsii/issue/IXI-14): Password-strength checks need a password creation/change flow. The repository currently provisions admin users outside the application.
- [IXI-4](https://linear.app/ixirsii/issue/IXI-4): Production forwarding-chain verification and the associated throttling policy remain open. The actual Cloudflare/Railway trust boundaries need deployment evidence; no proxy ranges or account-recovery policy were guessed.

## Validation

- 55 unit and PostgreSQL integration tests passed with locked dependencies and offline SQLx compilation.
- All four HTTP integration tests also passed in release mode, exercising production cookie names, headers, manifest icon routes, and public content filtering.
- Strict Clippy (`--all-targets -- -D warnings`) and formatting passed.
- SQLx metadata was regenerated and checked against the migrated local PostgreSQL 18 schema.
- Chromium checks covered public/admin layouts from 320 to 1440 pixels, keyboard theme choices, login, HTMX previews (including oversized-input feedback and recovery), related posts, and reduced motion. The generated RSS parsed as XML with zero errors. Maximum-length titles and tag names also wrap within a 320-pixel viewport.
- Semantic text colors passed 4.5:1 on Base, Mantle, and Surface 0 in all themes: minimum ratios were Latte 4.85, Frappe 4.80, Macchiato 5.79, and Mocha 6.14.
- The CI database script passed shell syntax checks and a disposable PostgreSQL 18 container probe covering startup, socket URL, database creation/deletion, and cleanup. The actual FreeBSD Forgejo job still needs a CI run.

## Code review

Used the repository's [CodeRabbit review skill](../.agents/skills/code-review/SKILL.md).
The first review identified a preview-validation response that HTMX would not swap;
that is fixed and browser-verified. The follow-up reviewed the complete diff,
including all new source files, and reported one minor suggestion: renew valid
login challenges halfway through their lifetime. Kept the fixed signed expiry so
opening another login page does not invalidate an existing tab's form. Expired
challenges already receive a replacement form token. No critical or warning-level
findings remain.

## Deployment note

Migration `20260923000003` adds the singleton login signing-key table. Startup applies it and initializes the key automatically. No new configuration is required. The key is secret data and is never sent to browsers; README documents storage and rotation. Existing application sessions keep using the persistent session store.
