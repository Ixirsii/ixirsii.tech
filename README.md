# Personal blog

[![Discord][badge-discord]][link-discord]
[![License][badge-bsd-3]][link-bsd-3]
[![Staging][badge-workflow-staging]][link-workflow-staging]

## Links

- [Production][production]
- [Beta][beta]
- [Release runner setup and image promotion](docs/image-releases.md)

## Tech stack

| Requirement     | Dependency                       |
| --------------- | -------------------------------- |
| Database        | [SQLx][sqlx]                     |
| HTTP handler    | [Axum][axum]                     |
| HTML templates  | [Askama][askama]                 |
| Markdown parser | [pulldown_cmark][pulldown-cmark] |
| Javascript      | [HTMX][htmx]                     |

## Todo

- Code review (un-vibe the code)
  - Clean up templates
  - Clean up styles
  - Clean up database
  - Clean up pages
  - Clean up CSRF
- Re-add observability with Grafana Cloud

[badge-discord]: https://img.shields.io/discord/452233740408717313?color=b4befe&label=discord&labelColor=1e1e2e&logo=discord&style=flat
[link-discord]: https://discord.gg/KTgshSbEUU
[badge-bsd-3]: https://img.shields.io/badge/License-BSD_3--Clause-89b4fa?labelColor=1e1e2e&logo=bsd&logoColor=white&style=flat
[link-bsd-3]: https://opensource.org/license/bsd-3-clause
[badge-workflow-staging]: https://git.9dev.tech/Ixirsii/ixirsii.tech/badges/workflows/staging.yml/badge.svg?branch=main&event=push&label=Staging&labelColor=1e1e2e&style=flat&logo=forgejo&logoColor=white
[link-workflow-staging]: https://git.9dev.tech/Ixirsii/ixirsii.tech/actions/workflows/staging.yml
[production]: https://ixirsii.tech
[beta]: https://beta.ixirsii.tech
[sqlx]: https://docs.rs/sqlx
[axum]: https://docs.rs/axum
[askama]: https://docs.rs/askama/
[pulldown-cmark]: https://docs.rs/pulldown-cmark
[htmx]: https://htmx.org
