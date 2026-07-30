//! Admin tag management route handlers.

use askama::Template;
use axum::{
    Form,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::{
    AdminAuthSession, AppState,
    csrf::{CsrfForm, csrf_token, rotate_csrf_token, verify_csrf_token},
    database::{
        TagWithCount, create_tag as create_tag_record, delete_tag as delete_tag_record,
        get_tag_by_id, list_tags_with_count, tag_name_exists, tag_name_exists_for_other_tag,
        tag_slug_exists, tag_slug_exists_for_other_tag, update_tag as update_tag_record,
    },
    page::slugify,
};

const ADMIN_TAGS_PATH: &str = "/admin/tags";

#[derive(Clone, Default)]
struct TagFormErrors {
    name: Option<String>,
    slug: Option<String>,
    security: Option<String>,
}

impl TagFormErrors {
    const fn has_errors(&self) -> bool {
        self.name.is_some() || self.slug.is_some() || self.security.is_some()
    }
}

struct TagEditFormState {
    id: u64,
    name: String,
    slug: String,
    errors: TagFormErrors,
}

#[derive(Template)]
#[template(path = "admin_tags_index.html")]
struct AdminTagsPage {
    title: &'static str,
    show_admin_link: bool,
    csrf_token: String,
    create_name: String,
    create_errors: TagFormErrors,
    tags: Vec<AdminTagView>,
}

struct AdminTagView {
    id: u64,
    name: String,
    slug: String,
    count: i32,
    errors: TagFormErrors,
}

#[derive(Deserialize)]
pub struct TagCreateForm {
    #[serde(default)]
    name: String,
    #[serde(default)]
    csrf_token: String,
}

#[derive(Deserialize)]
pub struct TagEditForm {
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    csrf_token: String,
}

/// Lists tags for admin management.
///
/// # Errors
/// Returns `500 Internal Server Error` if the database query or template render fails.
pub async fn tags_index(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
) -> Result<Html<String>, StatusCode> {
    let page = build_tags_page(
        &state,
        &auth_session,
        String::new(),
        TagFormErrors::default(),
        None,
    )
    .await?;

    Ok(Html(
        page.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

/// Creates a tag.
///
/// # Errors
/// Returns `500 Internal Server Error` if the database write or template render fails.
pub async fn create_tag(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Form(form): Form<TagCreateForm>,
) -> Result<Response, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return render_tags_response(
            &build_tags_page(
                &state,
                &auth_session,
                form.name,
                TagFormErrors {
                    security: Some(String::from(
                        "Security token expired. Refresh and try again.",
                    )),
                    ..TagFormErrors::default()
                },
                None,
            )
            .await?,
            StatusCode::FORBIDDEN,
        );
    }

    let (mut errors, name, slug) = validate_create_tag_form(&form);

    if errors.name.is_none() {
        if tag_name_exists(&(*state.db_pool), &name)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            errors.name = Some(format!("A tag named '{name}' already exists."));
        }
        if !slug.is_empty()
            && tag_slug_exists(&(*state.db_pool), &slug)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            errors.name = Some(format!(
                "The generated slug '{slug}' already exists. Choose a different name."
            ));
        }
    }

    if errors.has_errors() {
        return render_tags_response(
            &build_tags_page(&state, &auth_session, form.name, errors, None).await?,
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }

    create_tag_record(&(*state.db_pool), &name, &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    rotate_csrf_token(&auth_session.session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(ADMIN_TAGS_PATH).into_response())
}

/// Updates a tag.
///
/// # Errors
/// Returns `404 Not Found` if the tag doesn't exist,
/// or `500 Internal Server Error` if the database write or template render fails.
pub async fn edit_tag(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Path(tag_id): Path<u64>,
    Form(form): Form<TagEditForm>,
) -> Result<Response, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return render_tags_response(
            &build_tags_page(
                &state,
                &auth_session,
                String::new(),
                TagFormErrors::default(),
                Some(TagEditFormState {
                    id: tag_id,
                    name: form.name,
                    slug: form.slug,
                    errors: TagFormErrors {
                        security: Some(String::from(
                            "Security token expired. Refresh and try again.",
                        )),
                        ..TagFormErrors::default()
                    },
                }),
            )
            .await?,
            StatusCode::FORBIDDEN,
        );
    }

    if get_tag_by_id(&(*state.db_pool), tag_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_none()
    {
        return Err(StatusCode::NOT_FOUND);
    }

    let (mut errors, name, slug) = validate_edit_tag_form(&form);

    if errors.name.is_none()
        && tag_name_exists_for_other_tag(&(*state.db_pool), &name, tag_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        errors.name = Some(format!("A tag named '{name}' already exists."));
    }
    if errors.slug.is_none()
        && !slug.is_empty()
        && tag_slug_exists_for_other_tag(&(*state.db_pool), &slug, tag_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        errors.slug = Some(format!("The slug '{slug}' already exists."));
    }

    if errors.has_errors() {
        return render_tags_response(
            &build_tags_page(
                &state,
                &auth_session,
                String::new(),
                TagFormErrors::default(),
                Some(TagEditFormState {
                    id: tag_id,
                    name: form.name,
                    slug: form.slug,
                    errors,
                }),
            )
            .await?,
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }

    update_tag_record(&(*state.db_pool), tag_id, &name, &slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    rotate_csrf_token(&auth_session.session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(ADMIN_TAGS_PATH).into_response())
}

/// Deletes a tag.
///
/// # Errors
/// Returns `500 Internal Server Error` if the database delete fails.
pub async fn delete_tag(
    State(state): State<AppState>,
    auth_session: AdminAuthSession,
    Path(tag_id): Path<u64>,
    Form(form): Form<CsrfForm>,
) -> Result<Redirect, StatusCode> {
    if !verify_csrf_token(&auth_session.session, &form.csrf_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    delete_tag_record(&(*state.db_pool), tag_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    rotate_csrf_token(&auth_session.session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to(ADMIN_TAGS_PATH))
}

fn validate_create_tag_form(input: &TagCreateForm) -> (TagFormErrors, String, String) {
    let mut errors = TagFormErrors::default();
    let name = input.name.trim().to_string();
    let slug = if name.is_empty() {
        errors.name = Some("Tag name is required.".into());
        String::new()
    } else {
        let s = slugify(&name);
        if s.is_empty() {
            errors.name = Some(
                "The tag name must contain at least one letter or number so a slug can be generated."
                    .into(),
            );
        }
        s
    };

    (errors, name, slug)
}

fn validate_edit_tag_form(input: &TagEditForm) -> (TagFormErrors, String, String) {
    let mut errors = TagFormErrors::default();
    let name = input.name.trim().to_string();
    let slug = input.slug.trim().to_string();

    if name.is_empty() {
        errors.name = Some("Tag name is required.".into());
    }
    if slug.is_empty() {
        errors.slug = Some("Slug is required when editing a tag.".into());
    }

    (errors, name, slug)
}

async fn build_tags_page(
    state: &AppState,
    auth_session: &AdminAuthSession,
    create_name: String,
    create_errors: TagFormErrors,
    edit_state: Option<TagEditFormState>,
) -> Result<AdminTagsPage, StatusCode> {
    let tags = list_tags_with_count(&(*state.db_pool))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(AdminTagsPage {
        title: "Admin Tags",
        show_admin_link: auth_session.user.is_some(),
        csrf_token: csrf_token(&auth_session.session)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        create_name,
        create_errors,
        tags: build_tag_views(tags, edit_state.as_ref()),
    })
}

fn build_tag_views(
    tags: Vec<TagWithCount>,
    edit_state: Option<&TagEditFormState>,
) -> Vec<AdminTagView> {
    tags.into_iter()
        .map(|tag| {
            if let Some(edit) = edit_state
                && edit.id == tag.id
            {
                AdminTagView {
                    id: tag.id,
                    name: edit.name.clone(),
                    slug: edit.slug.clone(),
                    count: tag.count,
                    errors: edit.errors.clone(),
                }
            } else {
                AdminTagView {
                    id: tag.id,
                    name: tag.name,
                    slug: tag.slug,
                    count: tag.count,
                    errors: TagFormErrors::default(),
                }
            }
        })
        .collect()
}

fn render_tags_response(page: &AdminTagsPage, status: StatusCode) -> Result<Response, StatusCode> {
    Ok((
        status,
        Html(
            page.render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
    )
        .into_response())
}
