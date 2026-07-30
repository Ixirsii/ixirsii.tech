//! Error page route handlers and response types.

use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::AdminAuthSession;

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage<'a> {
    title: &'a str,
    show_admin_link: bool,
    status_code: u16,
    heading: &'a str,
    message: &'a str,
}

/// HTTP error that renders as a site-styled page.
pub struct PageError {
    status: StatusCode,
    show_admin_link: bool,
}

impl PageError {
    /// Creates a `404 Not Found` page error.
    #[must_use]
    pub const fn not_found(show_admin_link: bool) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            show_admin_link,
        }
    }

    /// Creates a `500 Internal Server Error` page error.
    #[must_use]
    pub const fn internal_server_error(show_admin_link: bool) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            show_admin_link,
        }
    }
}

impl IntoResponse for PageError {
    fn into_response(self) -> Response {
        let (title, heading, message) = match self.status {
            StatusCode::NOT_FOUND => (
                "404 Not Found",
                "Page not found",
                "The page you were looking for does not exist or has moved.",
            ),
            StatusCode::INTERNAL_SERVER_ERROR => (
                "500 Internal Server Error",
                "Something went wrong",
                "The server ran into a problem while rendering this page.",
            ),
            _ => (
                "Error",
                "Something went wrong",
                "The server could not complete this request.",
            ),
        };
        let page = ErrorPage {
            title,
            show_admin_link: self.show_admin_link,
            status_code: self.status.as_u16(),
            heading,
            message,
        };

        page.render().map_or_else(
            |_| (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response(),
            |html| (self.status, Html(html)).into_response(),
        )
    }
}

/// Renders the fallback page for unmatched routes.
pub async fn not_found(auth_session: AdminAuthSession) -> PageError {
    PageError::not_found(auth_session.user.is_some())
}
