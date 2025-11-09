use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};
use cuid2::cuid;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

use super::{file::File, Key};

#[derive(Debug, Serialize, Deserialize, Clone)]
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
            id: RecordId::from_table_key("comment", cuid()),
            commenter: RecordId::from_table_key("user", commenter.0),
            post: RecordId::from_table_key("post", post.0),
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
            id: Key(value.id.key().to_owned()),
            commenter_id: Key(value.commenter.key().to_owned()),
            post_id: Key(value.post.key().to_owned()),
            content: value.content,
            attachments: value.attachments,
            created_at: value.created_at.timestamp(),
            edited_at: value.edited_at.timestamp(),
        }
    }
}
