//! About route handler.

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};

use crate::{AdminAuthSession, AppState};

#[derive(Template)]
#[template(path = "about_page.html")]
struct AboutPage<'a> {
    title: &'a str,
    show_admin_link: bool,
}

/// Renders the about page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn about_page(
    State(_state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, StatusCode> {
    let page = AboutPage {
        title: "About",
        show_admin_link: auth_session.user.is_some(),
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
