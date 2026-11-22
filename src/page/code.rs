//! Code route handlers.

use askama::Template;
use axum::{extract::State, response::Html};
use pulldown_cmark::{Event, Parser, Tag, html};

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
    description: include_str!("../../content/dotfiles/README.md"),
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
    has_readme_heading: bool,
    name: &'static str,
    tagline: &'static str,
    description: String,
    repo_url: &'static str,
    website_url: Option<&'static str>,
    tech_tags: &'static [&'static str],
}

impl From<Project> for ProjectView {
    fn from(project: Project) -> Self {
        let has_readme_heading = project.slug == DOTFILES.slug;
        Self {
            has_readme_heading,
            name: project.name,
            description: if has_readme_heading {
                render_dotfiles_readme(project.description)
            } else {
                render_markdown(project.description)
            },
            repo_url: project.repo_url,
            tech_tags: project.tech_tags,
            tagline: project.tagline,
            website_url: project.website_url,
        }
    }
}

fn render_dotfiles_readme(readme: &str) -> String {
    // The upstream badge row is a paragraph of linked badge references after the title.
    let mut paragraphs = readme.splitn(3, "\n\n");
    let title = paragraphs.next().unwrap_or_default();
    let badges = paragraphs.next().unwrap_or_default();
    let body = if !badges.is_empty()
        && badges
            .lines()
            .all(|line| line.starts_with("[![") && line.contains("][badge-") && line.ends_with(']'))
    {
        format!("{title}\n\n{}", paragraphs.next().unwrap_or_default())
    } else {
        readme.to_owned()
    };
    let events = Parser::new(&body).map(|mut event| {
        if let Event::Start(Tag::Image { dest_url, .. }) = &mut event
            && dest_url.starts_with("screenshots/")
        {
            *dest_url = crate::static_assets::static_url(&format!("dotfiles/{dest_url}")).into();
        }
        event
    });
    let mut output = String::new();
    html::push_html(&mut output, events);
    ammonia::Builder::default()
        .add_tags(["details", "summary"])
        .clean(&output)
        .to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotfiles_page_preserves_readme_and_serves_local_screenshots() -> Result<(), askama::Error> {
        let page = CodeProjectPage {
            title: DOTFILES.name,
            show_admin_link: false,
            project: DOTFILES.into(),
        }
        .render()?;
        assert_eq!(page.matches("<h1>Dotfiles</h1>").count(), 1);
        assert!(page.contains("<details>") && page.contains("<summary>Screenshots</summary>"));
        let description = render_dotfiles_readme(DOTFILES.description);
        assert!(!description.contains("discord.gg") && !description.contains("shields.io"));
        let sources: Vec<_> = description
            .split("<img src=\"")
            .skip(1)
            .filter_map(|image| image.split('"').next())
            .collect();
        assert_eq!(
            sources.len(),
            DOTFILES.description.matches("](screenshots/").count()
        );
        assert_ne!(sources.len(), 0);
        assert!(sources.iter().all(|src| {
            crate::static_assets::source_path_for_request(src)
                .is_some_and(|path| path.starts_with("dotfiles/screenshots/"))
        }));
        Ok(())
    }
}
