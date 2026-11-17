//! Admin post management route handlers.

use std::collections::HashSet;

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::Form;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;

use crate::{
    AdminAuthSession, AppState,
    csrf::{CsrfForm, csrf_token, rotate_csrf_token, verify_csrf_token},
    database::{
        self, Post, PostSort, PostStatusFilter, Tag, count_posts, create_post_with_tags,
        get_post_by_id, list_posts, list_tags_for_post, list_tags_with_count, slug_exists,
        slug_exists_for_other_post, update_post_with_tags,
    },
    page::{
        PAGE_SIZE, render_markdown, slugify,
        validation::{
            POST_PREVIEW_MAX, POST_SLUG_MAX, POST_TITLE_MAX, body_error, slug_error, text_error,
        },
    },
};

/* ********************************************************************************************** *
 *                                           Constants                                            *
 * ********************************************************************************************** */

const ADMIN_POSTS_PATH: &str = "/admin/posts";

#[derive(Template)]
#[template(path = "admin_posts_index.html")]
struct AdminPostsIndexPage {
    title: &'static str,
    show_admin_link: bool,
    csrf_token: String,
    posts: Vec<AdminPostListItemView>,
    selected_filter: &'static str,
    selected_sort: &'static str,
    current_page: u32,
    total_pages: u32,
}

#[derive(Template)]
#[template(path = "admin_post_form.html")]
struct AdminPostFormPage {
    title: String,
    show_admin_link: bool,
    heading: &'static str,
    is_edit: bool,
    csrf_token: String,
    form_action: String,
    delete_action: Option<String>,
    form: PostFormView,
    errors: PostFormErrors,
}

#[derive(Template)]
#[template(path = "admin_markdown_preview.html")]
struct AdminMarkdownPreview {
    html: String,
    empty_message: String,
}

struct AdminPostListItemView {
    id: i64,
    title: String,
    slug: String,
    status_label: &'static str,
    status_class: &'static str,
    published_at: String,
    created_at: String,
    updated_at: String,
    tags: Vec<TagChipView>,
}

struct TagChipView {
    name: String,
}

struct TagOptionView {
    id: i64,
    name: String,
    slug: String,
    selected: bool,
}

#[derive(Default)]
struct PostFormErrors {
    title: Option<String>,
    slug: Option<String>,
    body: Option<String>,
    preview: Option<String>,
    published_at: Option<String>,
    security: Option<String>,
}

impl PostFormErrors {
    const fn has_errors(&self) -> bool {
        self.title.is_some()
            || self.slug.is_some()
            || self.body.is_some()
            || self.preview.is_some()
            || self.published_at.is_some()
            || self.security.is_some()
    }
}

struct PostFormView {
    title: String,
    slug: String,
    preview: String,
    body: String,
    published_at: String,
    tags: Vec<TagOptionView>,
    preview_html: String,
    body_html: String,
}

#[derive(Clone, Default, Deserialize)]
pub struct PostFormInput {
    #[serde(default)]
    title: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    preview: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    published_at: String,
    #[serde(default)]
    tag_ids: Vec<i64>,
    #[serde(default)]
    csrf_token: String,
}

#[derive(Deserialize)]
pub struct AdminPostsQuery {
    filter: Option<String>,
    sort: Option<String>,
    page: Option<String>,
}

#[derive(Deserialize)]
pub struct PreviewInput {
    #[serde(default)]
    preview: String,
    csrf_token: String,
}

#[derive(Deserialize)]
pub struct BodyInput {
    #[serde(default)]
    body: String,
    csrf_token: String,
}

/// Lists all posts for admin management.
///
/// # Errors
/// Returns `500 Internal Server Error` if the database query or template render fails.
pub async fn admin_posts_index(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Query(params): Query<AdminPostsQuery>,
) -> Result<Html<String>, StatusCode> {
    let filter = parse_filter(params.filter.as_deref());
    let sort = parse_sort(params.sort.as_deref());
    let requested_page = parse_page(params.page.as_deref());
    let total_posts = count_posts(&(*state.db_pool), &filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let total_pages = total_posts.div_ceil(PAGE_SIZE);
    let current_page = requested_page.min(total_pages.max(1));
    let post_rows = list_posts(&(*state.db_pool), &filter, current_page, PAGE_SIZE, &sort)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let page = AdminPostsIndexPage {
        title: "Admin Posts",
        show_admin_link: auth_session.user.is_some(),
        csrf_token: csrf_token(&auth_session.session)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        posts: build_admin_post_views(post_rows),
        selected_filter: filter_value(&filter),
        selected_sort: sort_value(&sort),
        current_page,
        total_pages,
    };

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Renders the new post page.
///
/// # Errors
/// Returns `500 Internal Server Error` if the database query or template render fails.
pub async fn new_post_page(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, StatusCode> {
    let page = build_post_form_page(
        &state,
        &auth_session,
        "New Post",
        None,
        false,
        PostFormInput::default(),
        PostFormErrors::default(),
    )
    .await?;

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Creates a new post.
///
/// # Errors
/// Returns `500 Internal Server Error` if the database write or template render fails.
pub async fn create_post(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Form(form): Form<PostFormInput>,
) -> Result<Response, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return render_post_form_response(
            &build_post_form_page(
                &state,
                &auth_session,
                "New Post",
                None,
                false,
                form,
                PostFormErrors {
                    security: Some(String::from(
                        "Security token expired. Refresh and try again.",
                    )),
                    ..PostFormErrors::default()
                },
            )
            .await?,
            StatusCode::FORBIDDEN,
        );
    }

    let (mut errors, title, body, slug) = validate_create_form(&form);

    let published_at = match parse_published_at(&form.published_at) {
        Ok(v) => v,
        Err(e) => {
            errors.published_at = Some(e);
            None
        }
    };

    if errors.title.is_none()
        && !slug.is_empty()
        && slug_exists(&(*state.db_pool), &slug)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        errors.title = Some(format!(
            "The generated slug '{slug}' already exists. Adjust the title and try again."
        ));
    }

    if errors.has_errors() {
        return render_post_form_response(
            &build_post_form_page(&state, &auth_session, "New Post", None, false, form, errors)
                .await?,
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }

    let tag_ids = filter_known_tag_ids(&state, &form.tag_ids).await?;
    let result = create_post_with_tags(
        &state.db_pool,
        &title,
        &slug,
        &body,
        optional_text(&form.preview),
        published_at,
        &tag_ids,
    )
    .await;
    let post_id = match result {
        Ok(id) => id,
        Err(error) => {
            let errors = post_conflict(&error, false).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
            return render_post_form_response(
                &build_post_form_page(&state, &auth_session, "New Post", None, false, form, errors)
                    .await?,
                StatusCode::CONFLICT,
            );
        }
    };

    rotate_csrf_token(&auth_session.session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("{ADMIN_POSTS_PATH}/{post_id}/edit")).into_response())
}

/// Renders the edit page for an existing post.
///
/// # Errors
/// Returns `404 Not Found` if the post doesn't exist, or `500 Internal Server Error`
/// if the database query or template render fails.
pub async fn edit_post_page(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Path(post_id): Path<i64>,
) -> Result<Html<String>, StatusCode> {
    let Some(post) = get_post_by_id(&(*state.db_pool), post_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        return Err(StatusCode::NOT_FOUND);
    };
    let tag_ids = list_tags_for_post(&(*state.db_pool), post_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|tag| tag.id)
        .collect();
    let form = PostFormInput {
        title: post.title,
        slug: post.slug,
        preview: post.preview.unwrap_or_default(),
        body: post.body,
        published_at: format_datetime_input(post.published_at),
        tag_ids,
        csrf_token: String::new(),
    };
    let page = build_post_form_page(
        &state,
        &auth_session,
        "Edit Post",
        Some(post_id),
        true,
        form,
        PostFormErrors::default(),
    )
    .await?;

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Updates an existing post.
///
/// # Errors
/// Returns `404 Not Found` if the post doesn't exist, or `500 Internal Server Error`
/// if the database write or template render fails.
pub async fn update_post(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Path(post_id): Path<i64>,
    Form(form): Form<PostFormInput>,
) -> Result<Response, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return render_post_form_response(
            &build_post_form_page(
                &state,
                &auth_session,
                "Edit Post",
                Some(post_id),
                true,
                form,
                PostFormErrors {
                    security: Some(String::from(
                        "Security token expired. Refresh and try again.",
                    )),
                    ..PostFormErrors::default()
                },
            )
            .await?,
            StatusCode::FORBIDDEN,
        );
    }

    if get_post_by_id(&(*state.db_pool), post_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_none()
    {
        return Err(StatusCode::NOT_FOUND);
    }

    let (mut errors, title, body, slug) = validate_edit_form(&form);

    let published_at = match parse_published_at(&form.published_at) {
        Ok(v) => v,
        Err(e) => {
            errors.published_at = Some(e);
            None
        }
    };

    if errors.slug.is_none()
        && !slug.is_empty()
        && slug_exists_for_other_post(&(*state.db_pool), &slug, post_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        errors.slug = Some(format!("The slug '{slug}' already exists."));
    }

    if errors.has_errors() {
        return render_post_form_response(
            &build_post_form_page(
                &state,
                &auth_session,
                "Edit Post",
                Some(post_id),
                true,
                form,
                errors,
            )
            .await?,
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }

    let tag_ids = filter_known_tag_ids(&state, &form.tag_ids).await?;
    if let Err(error) = update_post_with_tags(
        &state.db_pool,
        post_id,
        &title,
        &slug,
        &body,
        optional_text(&form.preview),
        published_at,
        &tag_ids,
    )
    .await
    {
        let errors = post_conflict(&error, true).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        return render_post_form_response(
            &build_post_form_page(
                &state,
                &auth_session,
                "Edit Post",
                Some(post_id),
                true,
                form,
                errors,
            )
            .await?,
            StatusCode::CONFLICT,
        );
    }

    rotate_csrf_token(&auth_session.session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(&format!("{ADMIN_POSTS_PATH}/{post_id}/edit")).into_response())
}

/// Deletes a post.
///
/// # Errors
/// Returns `500 Internal Server Error` if the database delete fails.
pub async fn delete_post(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Path(post_id): Path<i64>,
    Form(form): Form<CsrfForm>,
) -> Result<Redirect, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    database::delete_post(&(*state.db_pool), post_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    rotate_csrf_token(&auth_session.session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(ADMIN_POSTS_PATH))
}

/// Renders the live preview for the post preview field.
///
/// # Errors
/// Returns `500 Internal Server Error` if the template render fails.
pub async fn preview_post_preview(
    auth_session: AdminAuthSession,
    Form(form): Form<PreviewInput>,
) -> Result<Response, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return render_markdown_preview(
            "",
            "Preview unavailable until the page is refreshed.",
            StatusCode::FORBIDDEN,
        );
    }

    if let Some(error) = text_error(&form.preview, "Preview", POST_PREVIEW_MAX) {
        return render_markdown_preview("", &error, StatusCode::OK);
    }
    render_markdown_preview(
        &form.preview,
        "Write preview markdown to see it rendered here.",
        StatusCode::OK,
    )
}

/// Renders the live preview for the post body field.
///
/// # Errors
/// Returns `500 Internal Server Error` if the template render fails.
pub async fn preview_post_body(
    auth_session: AdminAuthSession,
    Form(form): Form<BodyInput>,
) -> Result<Response, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return render_markdown_preview(
            "",
            "Preview unavailable until the page is refreshed.",
            StatusCode::FORBIDDEN,
        );
    }

    if !form.body.trim().is_empty()
        && let Some(error) = body_error(&form.body)
    {
        return render_markdown_preview("", &error, StatusCode::OK);
    }
    render_markdown_preview(
        &form.body,
        "Write post body markdown to see it rendered here.",
        StatusCode::OK,
    )
}

fn post_conflict(error: &crate::Error, is_edit: bool) -> Option<PostFormErrors> {
    if error.unique_constraint() != Some("posts_slug_key") {
        return None;
    }
    let mut errors = PostFormErrors::default();
    if is_edit {
        errors.slug = Some("This slug already exists. Choose a different slug.".to_owned());
    } else {
        errors.title =
            Some("The generated slug already exists. Adjust the title and try again.".to_owned());
    }
    Some(errors)
}

fn validate_create_form(input: &PostFormInput) -> (PostFormErrors, String, String, String) {
    let mut errors = PostFormErrors {
        title: text_error(input.title.trim(), "Title", POST_TITLE_MAX),
        preview: text_error(input.preview.trim(), "Preview", POST_PREVIEW_MAX),
        body: body_error(&input.body),
        ..PostFormErrors::default()
    };
    let title = input.title.trim().to_string();
    let body = input.body.trim().to_string();

    if title.is_empty() {
        errors.title = Some("Title is required.".into());
    }
    if body.is_empty() {
        errors.body = Some("Body is required.".into());
    }

    let slug = if title.is_empty() {
        String::new()
    } else {
        let s = slugify(&title);
        if s.is_empty() {
            errors.title = Some(
                "The title must contain at least one letter or number so a slug can be generated."
                    .into(),
            );
        }
        if errors.title.is_none() {
            errors.title = slug_error(&s, POST_SLUG_MAX);
        }
        s
    };

    (errors, title, body, slug)
}

fn validate_edit_form(input: &PostFormInput) -> (PostFormErrors, String, String, String) {
    let mut errors = PostFormErrors {
        title: text_error(input.title.trim(), "Title", POST_TITLE_MAX),
        preview: text_error(input.preview.trim(), "Preview", POST_PREVIEW_MAX),
        body: body_error(&input.body),
        ..PostFormErrors::default()
    };
    let title = input.title.trim().to_string();
    let slug = input.slug.clone();
    let body = input.body.trim().to_string();

    if title.is_empty() {
        errors.title = Some("Title is required.".into());
    }
    errors.slug = slug_error(&slug, POST_SLUG_MAX);
    if body.is_empty() {
        errors.body = Some("Body is required.".into());
    }

    (errors, title, body, slug)
}

/* ********************************************************************************************** *
 *                                       Private functions                                        *
 * ********************************************************************************************** */

fn build_admin_post_views(rows: Vec<Post>) -> Vec<AdminPostListItemView> {
    rows.into_iter()
        .map(|row| {
            let (status_label, status_class) = post_status(row.published_at);

            AdminPostListItemView {
                id: row.id,
                title: row.title.clone(),
                slug: row.slug.clone(),
                status_label,
                status_class,
                published_at: format_optional_datetime(row.published_at, "Not scheduled"),
                created_at: format_datetime(row.created_at),
                updated_at: format_optional_datetime(row.updated_at, "Never"),
                tags: row
                    .tags
                    .iter()
                    .map(
                        |Tag {
                             id: _,
                             name,
                             slug: _,
                         }| TagChipView { name: name.clone() },
                    )
                    .collect(),
            }
        })
        .collect()
}

async fn build_post_form_page(
    state: &AppState,
    auth_session: &AdminAuthSession,
    heading: &'static str,
    post_id: Option<i64>,
    is_edit: bool,
    form: PostFormInput,
    errors: PostFormErrors,
) -> Result<AdminPostFormPage, StatusCode> {
    let tags = build_tag_options(&state.db_pool, &form.tag_ids).await?;

    Ok(AdminPostFormPage {
        title: heading.to_string(),
        show_admin_link: auth_session.user.is_some(),
        heading,
        is_edit,
        csrf_token: csrf_token(&auth_session.session)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        form_action: post_id.map_or_else(
            || ADMIN_POSTS_PATH.to_string(),
            |id| format!("{ADMIN_POSTS_PATH}/{id}"),
        ),
        delete_action: post_id.map(|id| format!("{ADMIN_POSTS_PATH}/{id}/delete")),
        form: PostFormView {
            title: form.title,
            slug: form.slug,
            preview_html: if errors.preview.is_none() {
                render_markdown(&form.preview)
            } else {
                String::new()
            },
            body_html: if errors.body.is_none() {
                render_markdown(&form.body)
            } else {
                String::new()
            },
            preview: form.preview,
            body: form.body,
            published_at: form.published_at,
            tags,
        },
        errors,
    })
}

async fn build_tag_options(
    pool: &sqlx::PgPool,
    selected_ids: &[i64],
) -> Result<Vec<TagOptionView>, StatusCode> {
    let selected: HashSet<_> = selected_ids.iter().copied().collect();
    let tags = list_tags_with_count(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(tags
        .into_iter()
        .map(|tag| TagOptionView {
            id: tag.id,
            name: tag.name,
            slug: tag.slug,
            selected: selected.contains(&tag.id),
        })
        .collect())
}

async fn filter_known_tag_ids(state: &AppState, tag_ids: &[i64]) -> Result<Vec<i64>, StatusCode> {
    let tags = list_tags_with_count(&(*state.db_pool))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let known_ids: HashSet<_> = tags.into_iter().map(|tag| tag.id).collect();
    let mut selected = HashSet::new();

    Ok(tag_ids
        .iter()
        .copied()
        .filter(|tag_id| known_ids.contains(tag_id) && selected.insert(*tag_id))
        .collect())
}

fn parse_filter(filter: Option<&str>) -> PostStatusFilter {
    match filter {
        Some("published") => PostStatusFilter::Published,
        Some("scheduled") => PostStatusFilter::Scheduled,
        Some("unscheduled") => PostStatusFilter::Unscheduled,
        _ => PostStatusFilter::All,
    }
}

const fn filter_value(filter: &PostStatusFilter) -> &'static str {
    match filter {
        PostStatusFilter::All => "all",
        PostStatusFilter::Published => "published",
        PostStatusFilter::Scheduled => "scheduled",
        PostStatusFilter::Unscheduled => "unscheduled",
    }
}

fn parse_sort(sort: Option<&str>) -> PostSort {
    match sort {
        Some("published_at_asc") => PostSort::PublishedAtAsc,
        Some("created_at_asc") => PostSort::CreatedAtAsc,
        Some("created_at_desc") => PostSort::CreatedAtDesc,
        _ => PostSort::PublishedAtDesc,
    }
}

fn parse_page(page: Option<&str>) -> u32 {
    page.and_then(|value| value.parse().ok())
        .filter(|page| *page > 0)
        .unwrap_or(1)
}

const fn sort_value(sort: &PostSort) -> &'static str {
    match sort {
        PostSort::PublishedAtDesc => "published_at_desc",
        PostSort::PublishedAtAsc => "published_at_asc",
        PostSort::CreatedAtDesc => "created_at_desc",
        PostSort::CreatedAtAsc => "created_at_asc",
    }
}

fn post_status(published_at: Option<DateTime<Utc>>) -> (&'static str, &'static str) {
    match published_at {
        None => ("Unscheduled", "status-unscheduled"),
        Some(timestamp) if timestamp > Utc::now() => ("Scheduled", "status-scheduled"),
        Some(_) => ("Published", "status-published"),
    }
}

fn format_datetime(datetime: DateTime<Utc>) -> String {
    datetime.format("%B %e, %Y %l:%M %p UTC").to_string()
}

fn format_optional_datetime(datetime: Option<DateTime<Utc>>, fallback: &str) -> String {
    datetime.map_or_else(|| fallback.to_string(), format_datetime)
}

fn format_datetime_input(datetime: Option<DateTime<Utc>>) -> String {
    datetime.map_or_default(|value| value.format("%Y-%m-%dT%H:%M").to_string())
}

fn optional_text(text: &str) -> Option<&str> {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_published_at(value: &str) -> Result<Option<DateTime<Utc>>, String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M")
        .map(|value| Some(value.and_utc()))
        .map_err(|_| String::from("Publish date must use the browser date-time picker value."))
}

fn render_post_form_response(
    page: &AdminPostFormPage,
    status: StatusCode,
) -> Result<Response, StatusCode> {
    Ok((
        status,
        Html(
            page.render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
    )
        .into_response())
}

fn render_markdown_preview(
    text: &str,
    empty_message: &str,
    status: StatusCode,
) -> Result<Response, StatusCode> {
    let preview = AdminMarkdownPreview {
        html: render_markdown(text),
        empty_message: empty_message.to_owned(),
    };

    Ok((
        status,
        Html(
            preview
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        extract::FromRequest,
        http::{Request, StatusCode, header::CONTENT_TYPE},
    };

    use super::{Form, PostFormInput, format_datetime_input, parse_published_at, post_status};

    #[tokio::test]
    async fn post_form_extracts_multiple_single_and_omitted_tags()
    -> Result<(), Box<dyn std::error::Error>> {
        for (tag_fields, expected_ids) in [
            ("&tag_ids=11&tag_ids=12", vec![11, 12]),
            ("&tag_ids=11", vec![11]),
            ("", vec![]),
        ] {
            let request = Request::builder()
                .method("POST")
                .uri("/admin/posts")
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "title=Test+Post&body=First%0ASecond&csrf_token=test-token{tag_fields}"
                )))?;
            let Form(form) = Form::<PostFormInput>::from_request(request, &()).await?;

            assert_eq!(form.tag_ids, expected_ids);
            assert_eq!(form.title, "Test Post");
            assert_eq!(form.body, "First\nSecond");
            assert_eq!(form.csrf_token, "test-token");
        }
        Ok(())
    }

    #[tokio::test]
    async fn post_form_rejects_invalid_tag_ids() -> Result<(), Box<dyn std::error::Error>> {
        for invalid_id in ["not-an-id", "9223372036854775808", ""] {
            let request = Request::builder()
                .method("POST")
                .uri("/admin/posts")
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("tag_ids=11&tag_ids={invalid_id}")))?;
            let Err(rejection) = Form::<PostFormInput>::from_request(request, &()).await else {
                return Err(format!("accepted invalid tag ID: {invalid_id}").into());
            };
            assert_eq!(rejection.status(), StatusCode::UNPROCESSABLE_ENTITY);
        }
        Ok(())
    }

    #[test]
    fn form_validation_enforces_lengths_and_slug_format() {
        let form = PostFormInput {
            title: "Valid title".into(),
            slug: "valid-title".into(),
            body: "Body".into(),
            ..PostFormInput::default()
        };
        assert!(!super::validate_edit_form(&form).0.has_errors());
        let oversized = PostFormInput {
            title: "a".repeat(256),
            preview: "é".repeat(513),
            body: "a".repeat(65_536),
            slug: "bad/slug".into(),
            ..form
        };
        let errors = super::validate_edit_form(&oversized).0;
        assert!(errors.title.is_some());
        assert!(errors.preview.is_some());
        assert!(errors.body.is_some());
        assert!(errors.slug.is_some());
        assert!(super::validate_create_form(&oversized).0.has_errors());
    }

    #[sqlx::test]
    async fn database_slug_conflicts_map_to_the_correct_form_field(
        pool: sqlx::PgPool,
    ) -> crate::Result<()> {
        use crate::database::{create_post_with_tags, update_post_with_tags};
        let first = create_post_with_tags(&pool, "First", "taken", "Body", None, None, &[]).await?;
        let second =
            create_post_with_tags(&pool, "Second", "available", "Body", None, None, &[]).await?;
        let Err(error) =
            create_post_with_tags(&pool, "Duplicate", "taken", "Body", None, None, &[]).await
        else {
            return Err(crate::Error::System("duplicate insert succeeded".into()));
        };
        assert!(super::post_conflict(&error, false).is_some_and(|errors| errors.title.is_some()));
        let Err(error) =
            update_post_with_tags(&pool, second, "Second", "taken", "Body", None, None, &[]).await
        else {
            return Err(crate::Error::System("duplicate update succeeded".into()));
        };
        assert!(super::post_conflict(&error, true).is_some_and(|errors| errors.slug.is_some()));
        assert_eq!(
            crate::database::get_post_by_id(&pool, first)
                .await?
                .map(|post| post.slug),
            Some("taken".to_owned())
        );
        assert_eq!(
            crate::database::get_post_by_id(&pool, second)
                .await?
                .map(|post| post.slug),
            Some("available".to_owned())
        );
        assert!(
            super::post_conflict(&crate::Error::Database(sqlx::Error::PoolClosed), false).is_none()
        );
        Ok(())
    }

    #[test]
    fn publication_input_round_trips_as_utc() -> Result<(), String> {
        let value = parse_published_at("2026-09-23T14:30")?;
        let timestamp = value.ok_or("expected publication time")?;
        assert_eq!(timestamp.to_rfc3339(), "2026-09-23T14:30:00+00:00");
        assert_eq!(format_datetime_input(value), "2026-09-23T14:30");
        Ok(())
    }

    #[test]
    fn publication_input_distinguishes_drafts_and_invalid_dates() -> Result<(), String> {
        assert_eq!(parse_published_at("  ")?, None);
        assert!(parse_published_at("2026-02-30T14:30").is_err());
        assert_eq!(post_status(None).0, "Unscheduled");
        assert_eq!(
            post_status(parse_published_at("2000-01-01T00:00")?).0,
            "Published"
        );
        assert_eq!(
            post_status(parse_published_at("2999-01-01T00:00")?).0,
            "Scheduled"
        );
        Ok(())
    }
}
