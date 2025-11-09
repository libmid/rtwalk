use std::time::SystemTime;

use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};
use cuid2::cuid;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

use super::{file::File, Key};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DBPost {
    pub id: RecordId,
    pub poster: RecordId,
    pub forum: RecordId,
    pub title: String,
    pub tags: Vec<String>,
    pub content: Option<String>,
    pub attachments: Vec<File>,
    pub created_at: DateTime<Utc>,
    pub edited_at: DateTime<Utc>,
    pub pinned: bool,
    pub locked: bool,
}

impl DBPost {
    pub fn new(
        title: String,
        tags: Vec<String>,
        content: Option<String>,
        attachments: Vec<File>,
        poster: Key,
        forum: Key,
    ) -> Self {
        let created_at: DateTime<Utc> = SystemTime::now().into();
        let edited_at = created_at.clone();
        Self {
            id: RecordId::from_table_key("forum", cuid()),
            poster: RecordId::from_table_key("user", poster.0),
            forum: RecordId::from_table_key("forum", forum.0),
            title,
            tags,
            content,
            attachments,
            created_at,
            edited_at,
            pinned: false,
            locked: false,
        }
    }
}

#[derive(SimpleObject, Debug, Serialize, Deserialize, Clone)]
pub struct Post {
    pub id: Key,
    pub poster_id: Key,
    pub forum_id: Key,
    pub title: String,
    pub tags: Vec<String>,
    pub content: Option<String>,
    pub attachments: Vec<File>,
    pub created_at: i64,
    pub edited_at: i64,
    pub pinned: bool,
    pub locked: bool,
}

impl From<DBPost> for Post {
    fn from(value: DBPost) -> Self {
        Self {
            id: Key(value.id.key().to_owned()),
            poster_id: Key(value.poster.key().to_owned()),
            forum_id: Key(value.forum.key().to_owned()),
            title: value.title,
            tags: value.tags,
            content: value.content,
            attachments: value.attachments,
            created_at: value.created_at.timestamp(),
            edited_at: value.edited_at.timestamp(),
            pinned: value.pinned,
            locked: value.locked,
        }
    }
}
