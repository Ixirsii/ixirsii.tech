//! Hall of Fame route handler.

use askama::Template;
use axum::{extract::State, response::Html};

use crate::{AdminAuthSession, AppState, page::PageError};

#[derive(Template)]
#[template(path = "hall-of-fame.html")]
struct HallOfFamePage<'a> {
    title: &'a str,
    show_admin_link: bool,
}

/// Renders the Hall of Fame page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn hall_of_fame_page(
    State(_state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, PageError> {
    let show_admin_link = auth_session.user.is_some();
    let page = HallOfFamePage {
        title: "Hall of Fame",
        show_admin_link,
    };

    Ok(Html(page.render().map_err(|_| {
        PageError::internal_server_error(show_admin_link)
    })?))
}
