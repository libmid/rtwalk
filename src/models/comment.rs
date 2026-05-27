use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};
use cuid2::cuid;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use super::{file::File, Key};

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct DBComment {
    pub id: RecordId,
    pub commenter: RecordId,
    pub post: RecordId,
    pub content: Option<String>,
    pub attachments: Vec<File>,
    pub created_at: DateTime<Utc>,
    pub edited_at: DateTime<Utc>,
}

impl DBComment {
    pub fn new(content: Option<String>, attachments: Vec<File>, commenter: Key, post: Key) -> Self {
        let created_at = DateTime::default();
        let edited_at = created_at.clone();
        Self {
            id: RecordId::new("comment", cuid()),
            commenter: RecordId::new("user", commenter.0),
            post: RecordId::new("post", post.0),
            content,
            attachments,
            created_at,
            edited_at,
        }
    }
}

#[derive(SimpleObject, Debug, Serialize, Deserialize, Clone)]
pub struct Comment {
    pub id: Key,
    pub commenter_id: Key,
    pub post_id: Key,
    pub content: Option<String>,
    pub attachments: Vec<File>,
    pub created_at: i64,
    pub edited_at: i64,
}

impl From<DBComment> for Comment {
    fn from(value: DBComment) -> Self {
        Self {
            id: Key(value.id.key),
            commenter_id: Key(value.commenter.key),
            post_id: Key(value.post.key),
            content: value.content,
            attachments: value.attachments,
            created_at: value.created_at.timestamp(),
            edited_at: value.edited_at.timestamp(),
        }
    }
}
