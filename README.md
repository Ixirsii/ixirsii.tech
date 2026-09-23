# ixirsii.tech

[![Discord](https://img.shields.io/discord/452233740408717313?color=738ADB&label=discord&logo=discord)][1]
[![License](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)][2]
[![Build](https://codeberg.org/Ixirsii/ixirsii.tech/badges/workflows/build.yml/badge.svg?branch=main&event=push&label=build)][3]

[Personal website][5].

## Tech stack

| Requirement     | Dependency           |
|-----------------|----------------------|
| Language        | [Rust][6]            |
| Database        | [SQLx][7]            |
| HTTP handler    | [Axum][8]            |
| HTML templates  | [Askama][9]          |
| Markdown parser | [pulldown_cmark][10] |
| Javascript      | [HTMX][11]           |

## CI

Pull requests and pushes to `main` run the security scans before the Rust build.
The build workflow calls `security.yml` and waits for all checks to succeed before
installing Tailwind or running Cargo. A failed scan or missing Socket API key blocks
the build; fork PRs cannot build because Forgejo withholds that key.

Release tags validate the release, run the same security and build sequence, and
then deploy to Railway. Scans and builds check out the same triggering commit.

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
