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
    page::{
        slugify,
        validation::{TAG_TEXT_MAX, slug_error, text_error},
    },
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
    id: i64,
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
    id: i64,
    name: String,
    slug: String,
    count: i64,
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

    if let Err(error) = create_tag_record(&(*state.db_pool), &name, &slug).await {
        let errors = tag_conflict(&error, false).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        return render_tags_response(
            &build_tags_page(&state, &auth_session, form.name, errors, None).await?,
            StatusCode::CONFLICT,
        );
    }

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
    Path(tag_id): Path<i64>,
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

    if let Err(error) = update_tag_record(&(*state.db_pool), tag_id, &name, &slug).await {
        let errors = tag_conflict(&error, true).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
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
            StatusCode::CONFLICT,
        );
    }

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
    Path(tag_id): Path<i64>,
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

fn tag_conflict(error: &crate::Error, is_edit: bool) -> Option<TagFormErrors> {
    let mut errors = TagFormErrors::default();
    match error.unique_constraint()? {
        "tags_name_key" => errors.name = Some("A tag with this name already exists.".to_owned()),
        "tags_slug_key" if is_edit => errors.slug = Some("This slug already exists.".to_owned()),
        "tags_slug_key" => {
            errors.name =
                Some("The generated slug already exists. Choose a different name.".to_owned());
        }
        _ => return None,
    }
    Some(errors)
}

fn tag_slug_error(slug: &str) -> Option<String> {
    if slug == "search" {
        Some(
            "The slug 'search' is reserved for tag search. Choose another name or slug.".to_owned(),
        )
    } else {
        slug_error(slug, TAG_TEXT_MAX)
    }
}

fn validate_create_tag_form(input: &TagCreateForm) -> (TagFormErrors, String, String) {
    let mut errors = TagFormErrors {
        name: text_error(input.name.trim(), "Tag name", TAG_TEXT_MAX),
        ..TagFormErrors::default()
    };
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
        if errors.name.is_none() {
            errors.name = tag_slug_error(&s);
        }
        s
    };

    (errors, name, slug)
}

fn validate_edit_tag_form(input: &TagEditForm) -> (TagFormErrors, String, String) {
    let mut errors = TagFormErrors {
        name: text_error(input.name.trim(), "Tag name", TAG_TEXT_MAX),
        ..TagFormErrors::default()
    };
    let name = input.name.trim().to_string();
    let slug = input.slug.clone();

    if name.is_empty() {
        errors.name = Some("Tag name is required.".into());
    }
    errors.slug = tag_slug_error(&slug);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_validation_enforces_schema_limits_and_slug_format() {
        let create = TagCreateForm {
            name: "a".repeat(128),
            csrf_token: String::new(),
        };
        assert!(validate_create_tag_form(&create).0.name.is_some());
        let edit = TagEditForm {
            name: "é".repeat(127),
            slug: "valid-tag".into(),
            csrf_token: String::new(),
        };
        assert!(!validate_edit_tag_form(&edit).0.has_errors());
        for slug in [
            "bad/slug",
            "tag#fragment",
            "tag?query",
            "UPPER",
            "two words",
            "-tag",
            "search",
        ] {
            let edit = TagEditForm {
                name: "Tag".into(),
                slug: slug.into(),
                csrf_token: String::new(),
            };
            assert!(validate_edit_tag_form(&edit).0.slug.is_some());
        }
    }

    #[sqlx::test]
    async fn database_name_and_slug_conflicts_map_to_form_fields(
        pool: sqlx::PgPool,
    ) -> crate::Result<()> {
        create_tag_record(&pool, "First", "first").await?;
        let second = create_tag_record(&pool, "Second", "second").await?;
        for (name, slug, expected_name) in [("First", "free", true), ("Third", "first", false)] {
            let Err(error) = create_tag_record(&pool, name, slug).await else {
                return Err(crate::Error::System("duplicate insert succeeded".into()));
            };
            assert!(tag_conflict(&error, false).is_some_and(|errors| errors.name.is_some()));
            let Err(error) = update_tag_record(&pool, second, name, slug).await else {
                return Err(crate::Error::System("duplicate update succeeded".into()));
            };
            let errors = tag_conflict(&error, true)
                .ok_or_else(|| crate::Error::System("conflict not recognized".into()))?;
            assert_eq!(errors.name.is_some(), expected_name);
            assert_eq!(errors.slug.is_some(), !expected_name);
        }
        assert!(tag_conflict(&crate::Error::Database(sqlx::Error::PoolClosed), true).is_none());
        Ok(())
    }
}
