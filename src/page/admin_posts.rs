//! Admin post management route handlers.

use std::collections::HashSet;

use askama::Template;
use axum::{
    Form,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use chrono::{Local, NaiveDateTime};
use serde::Deserialize;

use crate::{
    AdminAuthSession, AppState,
    csrf::{CsrfForm, csrf_token, rotate_csrf_token, verify_csrf_token},
    database::{
        self, Post, PostSort, PostStatusFilter, Tag, create_post_with_tags, get_post_by_id,
        list_posts, list_tags_for_post, list_tags_with_count, slug_exists,
        slug_exists_for_other_post, update_post_with_tags,
    },
    page::{PAGE_SIZE, render_markdown, slugify},
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
    empty_message: &'static str,
}

struct AdminPostListItemView {
    id: u64,
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
    id: u64,
    name: String,
    slug: String,
    selected: bool,
}

#[derive(Default)]
struct PostFormErrors {
    title: Option<String>,
    slug: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    security: Option<String>,
}

impl PostFormErrors {
    const fn has_errors(&self) -> bool {
        self.title.is_some()
            || self.slug.is_some()
            || self.body.is_some()
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
    tag_ids: Vec<u64>,
    #[serde(default)]
    csrf_token: String,
}

#[derive(Deserialize)]
pub struct AdminPostsQuery {
    filter: Option<String>,
    sort: Option<String>,
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
    let post_rows = list_posts(&(*state.db_pool), &filter, 1, PAGE_SIZE, &sort)
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
    let post_id = create_post_with_tags(
        &state.db_pool,
        &title,
        &slug,
        &body,
        optional_text(&form.preview),
        published_at,
        &tag_ids,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    Path(post_id): Path<u64>,
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
    Path(post_id): Path<u64>,
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
    update_post_with_tags(
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
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    Path(post_id): Path<u64>,
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

    render_markdown_preview(
        &form.body,
        "Write post body markdown to see it rendered here.",
        StatusCode::OK,
    )
}

fn validate_create_form(input: &PostFormInput) -> (PostFormErrors, String, String, String) {
    let mut errors = PostFormErrors::default();
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
        s
    };

    (errors, title, body, slug)
}

fn validate_edit_form(input: &PostFormInput) -> (PostFormErrors, String, String, String) {
    let mut errors = PostFormErrors::default();
    let title = input.title.trim().to_string();
    let slug = input.slug.trim().to_string();
    let body = input.body.trim().to_string();

    if title.is_empty() {
        errors.title = Some("Title is required.".into());
    }
    if slug.is_empty() {
        errors.slug = Some("Slug is required when editing a post.".into());
    }
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
    post_id: Option<u64>,
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
            preview_html: render_markdown(&form.preview),
            body_html: render_markdown(&form.body),
            preview: form.preview,
            body: form.body,
            published_at: form.published_at,
            tags,
        },
        errors,
    })
}

async fn build_tag_options(
    pool: &sqlx::MySqlPool,
    selected_ids: &[u64],
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

async fn filter_known_tag_ids(state: &AppState, tag_ids: &[u64]) -> Result<Vec<u64>, StatusCode> {
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

const fn sort_value(sort: &PostSort) -> &'static str {
    match sort {
        PostSort::PublishedAtDesc => "published_at_desc",
        PostSort::PublishedAtAsc => "published_at_asc",
        PostSort::CreatedAtDesc => "created_at_desc",
        PostSort::CreatedAtAsc => "created_at_asc",
    }
}

fn post_status(published_at: Option<NaiveDateTime>) -> (&'static str, &'static str) {
    match published_at {
        None => ("Unscheduled", "status-unscheduled"),
        Some(timestamp) if timestamp > Local::now().naive_local() => {
            ("Scheduled", "status-scheduled")
        }
        Some(_) => ("Published", "status-published"),
    }
}

fn format_datetime(datetime: NaiveDateTime) -> String {
    datetime.format("%B %e, %Y %l:%M %p").to_string()
}

fn format_optional_datetime(datetime: Option<NaiveDateTime>, fallback: &str) -> String {
    datetime.map_or_else(|| fallback.to_string(), format_datetime)
}

fn format_datetime_input(datetime: Option<NaiveDateTime>) -> String {
    datetime
        .map(|value| value.format("%Y-%m-%dT%H:%M").to_string())
        .unwrap_or_default()
}

fn optional_text(text: &str) -> Option<&str> {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_published_at(value: &str) -> Result<Option<NaiveDateTime>, String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M")
        .map(Some)
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
    empty_message: &'static str,
    status: StatusCode,
) -> Result<Response, StatusCode> {
    let preview = AdminMarkdownPreview {
        html: render_markdown(text),
        empty_message,
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
