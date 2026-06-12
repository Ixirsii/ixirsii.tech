//! Code route handlers.

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};

use crate::{AdminAuthSession, AppState, page::render_markdown};

/* ********************************************************************************************** *
 *                                           Constants                                            *
 * ********************************************************************************************** */

const ALL_PROJECTS: &[Project] = &[
    Project {
        slug: "randstring",
        name: "Randstring",
        tagline: "A random string generator for Odin.",
        description: concat!(
            "A simple library for generating random strings in [Odin](https://odin-lang.org/).\n\n",
            "Supports configurable character sets, length, and multiple generation modes.\n",
            "Designed to be simple, fast, and easy to integrate into any Odin project.",
        ),
        repo_url: "https://codeberg.org/Ixirsii/randstring",
        website_url: None,
        tech_tags: &["Odin"],
    },
    Project {
        slug: "dotfiles",
        name: "Dotfiles",
        tagline: "Personal configuration files.",
        description: concat!(
            "My personal dotfiles managed with GNU Stow.\n\n",
            "Includes configurations for:\n\n",
            "- **Neovim** — Custom LSP setup, keybindings, and plugin configuration\n",
            "- **WezTerm** — Terminal emulator configuration with Catppuccin theme\n",
            "- **zsh** — Shell aliases, functions, and prompt customization\n",
            "- **Git** — Global gitconfig and aliases\n\n",
            "All configurations are optimized for my daily workflow and designed to be",
            " easily reproducible on a new system.",
        ),
        repo_url: "https://codeberg.org/Ixirsii/dotfiles",
        website_url: None,
        tech_tags: &["Lua", "Shell", "Zsh", "Neovim"],
    },
    Project {
        slug: "ixirsii-tech",
        name: "ixirsii.tech",
        tagline: "This website.",
        description: concat!(
            "My personal website built with [Rust](https://rust-lang.org/) and [Axum](https://github.com/tokio-rs/axum).\n\n",
            "Features:\n\n",
            "- Server-side rendered HTML with [Askama](https://github.com/askama-rs/askama) templates\n",
            "- Blog with Markdown content stored in MySQL\n",
            "- [HTMX](https://htmx.org/) for dynamic interactions without JavaScript frameworks\n",
            "- [Tailwind CSS](https://tailwindcss.com/) v4 with Catppuccin Mocha theme\n",
            "- Dark/light theme switcher with multiple Catppuccin flavors\n",
        ),
        repo_url: "https://codeberg.org/Ixirsii/ixirsii.tech",
        website_url: Some("https://ixirsii.tech"),
        tech_tags: &["Rust", "Axum", "HTMX", "Tailwind CSS", "MySQL"],
    },
];

#[derive(Template)]
#[template(path = "code_page.html")]
struct CodePage<'a> {
    title: &'a str,
    show_admin_link: bool,
    projects: &'a [Project],
}

#[derive(Template)]
#[template(path = "code_project.html")]
struct CodeProjectPage {
    title: String,
    show_admin_link: bool,
    project: ProjectView,
}

struct Project {
    pub slug: &'static str,
    pub name: &'static str,
    pub tagline: &'static str,
    pub description: &'static str,
    pub repo_url: &'static str,
    pub website_url: Option<&'static str>,
    pub tech_tags: &'static [&'static str],
}

struct ProjectView {
    name: &'static str,
    tagline: &'static str,
    description: String,
    repo_url: &'static str,
    website_url: Option<&'static str>,
    tech_tags: &'static [&'static str],
}

/// Renders the code page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn code_index(
    State(_state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, StatusCode> {
    let page = CodePage {
        title: "Code",
        show_admin_link: auth_session.user.is_some(),
        projects: ALL_PROJECTS,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Renders an individual project page.
///
/// # Errors
/// Returns `404 Not Found` if the slug doesn't match any project, or
/// `500 Internal Server Error` if template rendering fails.
pub async fn code_project(
    State(_state): State<AppState>,
    auth_session: AdminAuthSession,
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let Some(project) = project_by_slug(&slug) else {
        return Err(StatusCode::NOT_FOUND);
    };

    let page = CodeProjectPage {
        title: project.name.to_string(),
        show_admin_link: auth_session.user.is_some(),
        project: ProjectView {
            name: project.name,
            tagline: project.tagline,
            description: render_markdown(project.description),
            repo_url: project.repo_url,
            website_url: project.website_url,
            tech_tags: project.tech_tags,
        },
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/* ********************************************************************************************** *
 *                                       Private functions                                        *
 * ********************************************************************************************** */

fn project_by_slug(slug: &str) -> Option<&'static Project> {
    ALL_PROJECTS.iter().find(|p| p.slug == slug)
}
