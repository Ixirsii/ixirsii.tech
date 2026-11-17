//! Database type definitions.
use chrono::{DateTime, Utc};

/* ********************************************************************************************** *
 *                                              Post                                              *
 * ********************************************************************************************** */

/// Post data.
#[derive(Debug)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub preview: Option<String>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
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

/* ********************************************************************************************** *
 *                                              Tag                                               *
 * ********************************************************************************************** */

/// Tag data.
#[derive(Clone, Debug)]
pub struct Tag {
    /// Tag ID.
    pub id: i64,
    /// Tag name.
    pub name: String,
    /// Tag slug.
    pub slug: String,
}

/* ********************************************************************************************** *
 *                                         Tag with count                                         *
 * ********************************************************************************************** */

/// Tag data with usage count.
#[derive(Clone, Debug)]
pub struct TagWithCount {
    /// Tag ID.
    pub id: i64,
    /// Tag name.
    pub name: String,
    /// Tag slug.
    pub slug: String,
    /// Tag usage count.
    pub count: i64,
}
