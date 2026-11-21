//! About and license notice route handlers.

use askama::Template;
use axum::{extract::State, response::Html};
use nine_dev_site_lib::assets::{Asset, icons};

use crate::{AdminAuthSession, AppState, page::PageError};

#[derive(Template)]
#[template(path = "about_page.html")]
struct AboutPage<'a> {
    title: &'a str,
    show_admin_link: bool,
    link_groups: &'static [LinkGroup],
}

struct LinkGroup {
    heading: &'static str,
    wide: bool,
    profiles: &'static [ProfileLink],
}

struct ProfileLink {
    href: &'static str,
    service: &'static str,
    handle: &'static str,
    icon: &'static Asset,
}

impl ProfileLink {
    const fn shared(
        href: &'static str,
        service: &'static str,
        handle: &'static str,
        icon: &'static Asset,
    ) -> Self {
        Self {
            href,
            service,
            handle,
            icon,
        }
    }
}

const LINK_GROUPS: &[LinkGroup] = &[
    LinkGroup {
        heading: "Connect",
        wide: true,
        profiles: &[
            ProfileLink::shared(
                "https://bsky.app/profile/ixirsii.tech",
                "Bluesky",
                "@ixirsii.tech",
                &icons::BLUESKY,
            ),
            ProfileLink::shared(
                "https://discord.gg/KTgshSbEUU",
                "Discord",
                "9 Developers",
                &icons::DISCORD,
            ),
            ProfileLink::shared(
                "https://www.instagram.com/ixirsii/",
                "Instagram",
                "Ixirsii",
                &icons::INSTAGRAM,
            ),
            ProfileLink::shared(
                "https://mastodon.social/@ixirsii",
                "Mastodon",
                "@ixirsii@mastodon.social",
                &icons::MASTODON,
            ),
            ProfileLink::shared(
                "https://signal.me/#eu/dMX2Io7JjngIRyfRlt4jMNRwgWK6wLlUMwqV_gLtmGitoUM69DlJS5npSHD3yaj9",
                "Signal",
                "Ixirsii.99",
                &icons::SIGNAL,
            ),
            ProfileLink::shared("https://x.com/ixirsii", "X", "@ixirsii", &icons::X),
        ],
    },
    LinkGroup {
        heading: "Development",
        wide: false,
        profiles: &[
            ProfileLink {
                href: "https://github.com/Ixirsii",
                service: "GitHub",
                handle: "Ixirsii",
                icon: &icons::GITHUB,
            },
            ProfileLink::shared(
                "https://9dev.tech",
                "9 Developers",
                "9dev.tech",
                &icons::WEBSITE,
            ),
        ],
    },
    LinkGroup {
        heading: "Interests",
        wide: false,
        profiles: &[
            ProfileLink::shared(
                "https://anilist.co/user/Ixirsii",
                "AniList",
                "Ixirsii",
                &icons::ANILIST,
            ),
            ProfileLink::shared(
                "https://open.spotify.com/user/shadowhawk54",
                "Spotify",
                "shadowhawk54",
                &icons::SPOTIFY,
            ),
        ],
    },
];

#[derive(Template)]
#[template(path = "licenses_page.html")]
struct LicensesPage {
    title: &'static str,
    show_admin_link: bool,
}

/// Renders the about page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn about_page(
    State(_state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let page = AboutPage {
        title: "About",
        show_admin_link,
        link_groups: LINK_GROUPS,
    };

    Ok(Html(page.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
}

/// Renders the third-party asset licenses.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn licenses_page(auth_session: AdminAuthSession) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let page = LicensesPage {
        title: "Third-party asset licenses",
        show_admin_link,
    };

    Ok(Html(page.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_profiles_use_shared_symbols_and_link_to_notices() -> Result<(), askama::Error> {
        let html = AboutPage {
            title: "About",
            show_admin_link: false,
            link_groups: LINK_GROUPS,
        }
        .render()?;
        let links = html
            .split_once("class=\"about-link-groups\"")
            .map_or("", |(_, links)| links);

        for icon in [
            &icons::BLUESKY,
            &icons::DISCORD,
            &icons::INSTAGRAM,
            &icons::MASTODON,
            &icons::SIGNAL,
            &icons::X,
            &icons::WEBSITE,
            &icons::ANILIST,
            &icons::SPOTIFY,
        ] {
            assert!(links.contains(&format!("<use href=\"{}#icon\">", icon.url)));
        }
        assert!(links.contains("class=\"about-link-service\">Codeberg</span>"));
        assert!(links.contains("href=\"/licenses\""));
        Ok(())
    }

    #[test]
    fn licenses_include_shared_asset_notices() -> Result<(), askama::Error> {
        let html = LicensesPage {
            title: "Third-party asset licenses",
            show_admin_link: false,
        }
        .render()?;

        assert!(html.contains("Copyright (c) Microsoft Corporation."));
        assert!(html.contains("Copyright (c) 2019-2024 The Bootstrap Authors"));
        Ok(())
    }
}
