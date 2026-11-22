//! Code route handlers.

use askama::Template;
use axum::{extract::State, response::Html};

use crate::{
    AdminAuthSession, AppState,
    page::{PageError, render_markdown},
};

/* ********************************************************************************************** *
 *                                           Constants                                            *
 * ********************************************************************************************** */

const DOTFILES: Project = Project {
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
    repo_url: "https://github.com/Ixirsii/dotfiles",
    website_url: None,
    tech_tags: &["Lua", "Shell", "Zsh", "Neovim"],
};

const IXIRSII_TECH: Project = Project {
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
    repo_url: "https://github.com/Ixirsii/ixirsii.tech",
    website_url: Some("https://ixirsii.tech"),
    tech_tags: &["Rust", "Axum", "HTMX", "Tailwind CSS", "MySQL"],
};

const RANDSTRING: Project = Project {
    slug: "randstring",
    name: "Randstring",
    tagline: "A random string generator for Odin.",
    description: concat!(
        "A simple library for generating random strings in [Odin](https://odin-lang.org/).\n\n",
        "Supports configurable character sets, length, and multiple generation modes.\n",
        "Designed to be simple, fast, and easy to integrate into any Odin project.",
    ),
    repo_url: "https://github.com/Ixirsii/Randstring",
    website_url: None,
    tech_tags: &["Odin"],
};

/* ********************************************************************************************** *
 *                                             Types                                              *
 * ********************************************************************************************** */

#[derive(Template)]
#[template(path = "code_page.html")]
struct CodePage<'a> {
    title: &'a str,
    show_admin_link: bool,
    projects: &'a [&'a Project],
}

#[derive(Template)]
#[template(path = "code_project.html")]
struct CodeProjectPage<'a> {
    title: &'a str,
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

impl From<Project> for ProjectView {
    fn from(project: Project) -> Self {
        Self {
            name: project.name,
            description: render_markdown(project.description),
            repo_url: project.repo_url,
            tech_tags: project.tech_tags,
            tagline: project.tagline,
            website_url: project.website_url,
        }
    }
}

/// Renders the code page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn code_index(
    State(_state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let page = CodePage {
        title: "Code",
        show_admin_link,
        projects: &[&DOTFILES, &IXIRSII_TECH, &RANDSTRING],
    };

    Ok(Html(page.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
}

pub async fn code_dotfiles(
    State(_state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let page = CodeProjectPage {
        title: DOTFILES.name,
        show_admin_link,
        project: DOTFILES.into(),
    };

    Ok(Html(page.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
}

pub async fn code_ixirsii_tech(
    State(_state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let page = CodeProjectPage {
        title: IXIRSII_TECH.name,
        show_admin_link,
        project: IXIRSII_TECH.into(),
    };

    Ok(Html(page.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
}

pub async fn code_randstring(
    State(_state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let page = CodeProjectPage {
        title: RANDSTRING.name,
        show_admin_link,
        project: RANDSTRING.into(),
    };

    Ok(Html(page.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
}

/* ********************************************************************************************** *
 *                                       Private functions                                        *
 * ********************************************************************************************** */
