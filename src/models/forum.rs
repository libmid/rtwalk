use std::time::SystemTime;

use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};
use cuid2::cuid;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

use super::{file::File, Key};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DBForum {
    pub id: RecordId,
    pub owner: RecordId,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub icon: Option<File>,
    pub banner: Option<File>,
    pub created_at: DateTime<Utc>,
    pub locked: bool,
}

impl DBForum {
    pub fn new(name: &str, owner: Key) -> Self {
        Self {
            id: RecordId::from_table_key("forum", cuid()),
            owner: RecordId::from_table_key("user", owner.0),
            name: name.to_string(),
            display_name: name.to_string(),
            description: None,
            icon: None,
            banner: None,
            created_at: SystemTime::now().into(),
            locked: false,
        }
    }
}

#[derive(SimpleObject, Debug)]
pub struct Forum {
    pub id: Key,
    pub owner_id: Key,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub icon: Option<File>,
    pub banner: Option<File>,
    pub created_at: i64,
    pub locked: bool,
}

impl From<DBForum> for Forum {
    fn from(value: DBForum) -> Self {
        Self {
            id: Key(value.id.key().to_owned()),
            owner_id: Key(value.owner.key().to_owned()),
            name: value.name,
            display_name: value.display_name,
            description: value.description,
            icon: value.icon,
            banner: value.banner,
            created_at: value.created_at.timestamp(),
            locked: value.locked,
        }
    }
}
