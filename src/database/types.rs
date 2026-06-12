//! Database type definitions.
use chrono::NaiveDateTime;
use sqlx::FromRow;

/* ********************************************************************************************** *
 *                                              Post                                              *
 * ********************************************************************************************** */

/// Post data.
#[derive(Debug)]
pub struct Post {
    pub id: u64,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub preview: Option<String>,
    pub created_at: NaiveDateTime,
    pub published_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub tags: Vec<Tag>,
}

/* ********************************************************************************************** *
 *                                           Post sort                                            *
 * ********************************************************************************************** */

/// Post sorting options for admin dashboard.
#[derive(Debug, Eq, PartialEq)]
pub enum PostSort {
    PublishedAtDesc,
    PublishedAtAsc,
    CreatedAtDesc,
    CreatedAtAsc,
}

impl SqlClause for PostSort {
    fn get_sql_clause(&self) -> String {
        use PostSort::{CreatedAtAsc, CreatedAtDesc, PublishedAtAsc, PublishedAtDesc};

        match self {
            PublishedAtDesc => "p.published_at DESC".to_string(),
            PublishedAtAsc => "p.published_at ASC".to_string(),
            CreatedAtDesc => "p.created_at DESC".to_string(),
            CreatedAtAsc => "p.created_at ASC".to_string(),
        }
    }
}

/* ********************************************************************************************** *
 *                                       Post status filter                                       *
 * ********************************************************************************************** */

/// Post filter options for admin dashboard.
#[derive(Debug, Eq, PartialEq)]
pub enum PostStatusFilter {
    All,
    Published,
    Scheduled,
    Unscheduled,
}

impl SqlClause for PostStatusFilter {
    fn get_sql_clause(&self) -> String {
        use PostStatusFilter::{All, Published, Scheduled, Unscheduled};

        match self {
            All => String::new(),
            Published => "WHERE p.published_at IS NOT NULL AND p.published_at <= NOW()".to_string(),
            Scheduled => "WHERE p.published_at IS NOT NULL AND p.published_at > NOW()".to_string(),
            Unscheduled => "WHERE p.published_at IS NULL".to_string(),
        }
    }
}

/* ********************************************************************************************** *
 *                                           Sql clause                                           *
 * ********************************************************************************************** */

/// Trait for types which can be converted into SQL clauses.
pub trait SqlClause {
    /// Get SQL clause as String.
    fn get_sql_clause(&self) -> String;
}

/* ********************************************************************************************** *
 *                                              Tag                                               *
 * ********************************************************************************************** */

/// Tag data.
#[derive(Clone, Debug, FromRow)]
pub struct Tag {
    /// Tag ID.
    pub id: u64,
    /// Tag name.
    pub name: String,
    /// Tag slug.
    pub slug: String,
}

/* ********************************************************************************************** *
 *                                         Tag with count                                         *
 * ********************************************************************************************** */

/// Tag data with usage count.
#[derive(Clone, Debug, FromRow)]
pub struct TagWithCount {
    /// Tag ID.
    pub id: u64,
    /// Tag name.
    pub name: String,
    /// Tag slug.
    pub slug: String,
    /// Tag usage count.
    pub count: i32,
}
