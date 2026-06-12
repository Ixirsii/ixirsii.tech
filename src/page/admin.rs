//! Admin and authentication route handlers.

use askama::Template;
use axum::{
    Form,
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    AdminAuthSession,
    csrf::{CsrfForm, csrf_token, rotate_csrf_token, verify_csrf_token},
    page::redirect_target,
};

#[derive(Template)]
#[template(path = "admin.html")]
struct AdminPage<'a> {
    title: &'a str,
    show_admin_link: bool,
    username: &'a str,
    csrf_token: &'a str,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage<'a> {
    title: &'a str,
    show_admin_link: bool,
    next: &'a str,
    username: &'a str,
    error: Option<&'a str>,
    error_is_security: bool,
    csrf_token: &'a str,
}

#[derive(Deserialize)]
pub struct LoginQuery {
    next: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
    next: Option<String>,
    csrf_token: String,
}

/// Renders the hidden login page.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn login_page(
    auth_session: AdminAuthSession,
    Query(params): Query<LoginQuery>,
) -> Result<Response, StatusCode> {
    if auth_session.user.is_some() {
        return Ok(Redirect::to(redirect_target(params.next.as_deref())).into_response());
    }

    render_login_page(
        &auth_session.session,
        StatusCode::OK,
        redirect_target(params.next.as_deref()),
        "",
        None,
        false,
    )
    .await
}

/// Authenticates the seeded admin user.
///
/// # Errors
/// Returns `500 Internal Server Error` if authentication or template rendering fails.
pub async fn login_submit(
    mut auth_session: AdminAuthSession,
    Form(form): Form<LoginForm>,
) -> Result<Response, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return render_login_page(
            &auth_session.session,
            StatusCode::FORBIDDEN,
            redirect_target(form.next.as_deref()),
            &form.username,
            Some("Security token expired. Refresh and try again."),
            true,
        )
        .await;
    }

    let redirect_to = redirect_target(form.next.as_deref());
    let credentials = crate::Credentials {
        username: form.username.clone(),
        password: form.password,
    };

    let Some(user) = auth_session
        .authenticate(credentials)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        return render_login_page(
            &auth_session.session,
            StatusCode::UNAUTHORIZED,
            redirect_to,
            &form.username,
            Some("Invalid username or password."),
            false,
        )
        .await;
    };

    auth_session
        .login(&user)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    rotate_csrf_token(&auth_session.session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(redirect_to).into_response())
}

/// Renders the protected admin dashboard.
///
/// # Errors
/// Returns `500 Internal Server Error` if template rendering fails.
pub async fn admin_dashboard(auth_session: AdminAuthSession) -> Result<Html<String>, StatusCode> {
    let csrf_token = csrf_token(&auth_session.session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let username = auth_session
        .user
        .as_ref()
        .map_or("", |user| user.username.as_str());
    let page = AdminPage {
        title: "Admin",
        show_admin_link: auth_session.user.is_some(),
        username,
        csrf_token: &csrf_token,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Logs the admin user out.
///
/// # Errors
/// Returns `500 Internal Server Error` if logout fails.
pub async fn logout(
    mut auth_session: AdminAuthSession,
    Form(form): Form<CsrfForm>,
) -> Result<Redirect, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    auth_session
        .logout()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/"))
}

async fn render_login_page(
    session: &Session,
    status: StatusCode,
    next: &str,
    username: &str,
    error: Option<&str>,
    error_is_security: bool,
) -> Result<Response, StatusCode> {
    let csrf_token = csrf_token(session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let page = LoginPage {
        title: "Login",
        show_admin_link: false,
        next,
        username,
        error,
        error_is_security,
        csrf_token: &csrf_token,
    };

    Ok((
        status,
        Html(
            page.render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
    )
        .into_response())
}
