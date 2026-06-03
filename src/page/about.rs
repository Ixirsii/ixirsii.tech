//! About route handler.

use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};

use crate::AppState;

#[derive(Template)]
#[template(path = "about_page.html")]
struct AboutPage<'a> {
    title: &'a str,
}

/// Renders the about page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn about_page(State(_state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let page = AboutPage { title: "About" };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}
