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

## Todo

- Code review (un-vibe the code)
  - Clean up templates
  - Clean up styles
  - Clean up database
  - Clean up pages
  - Clean up CSRF
- Re-add observability with Grafana Cloud
