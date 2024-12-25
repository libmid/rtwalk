use super::{file::File, Key};
use async_graphql::SimpleObject;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DBUser {
    pub id: RecordId,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub pfp: Option<File>,
    pub banner: Option<File>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub admin: bool,
    pub bot: bool,
    pub owner: Option<RecordId>,
}

impl DBUser {
    /// Creates a new [`DBUser`].
    pub fn new(username: String, bot: bool, owner: Option<RecordId>) -> Self {
        let created_at = DateTime::default();
        let modified_at = created_at.clone();
        DBUser {
            id: RecordId::from(("user".to_owned(), cuid2::cuid())),
            username: username.clone(),
            display_name: username.clone(),
            bio: None,
            pfp: None,
            banner: None,
            created_at,
            modified_at,
            admin: false,
            bot,
            owner,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DBUserSecret {
    pub user: RecordId,
    pub email: String,
    pub password: String,
    pub banned: bool,
}

#[derive(SimpleObject, Serialize, Deserialize, Debug)]
#[graphql(complex)]
pub struct User {
    pub id: Key,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub pfp: Option<File>,
    pub banner: Option<File>,
    pub created_at: i64,
    pub modified_at: i64,
    pub admin: bool,
    pub bot: bool,
    pub owner: Option<Key>,
}

impl From<DBUser> for User {
    fn from(value: DBUser) -> Self {
        Self {
            id: Key(value.id.key().to_owned()),
            username: value.username,
            display_name: value.display_name,
            bio: value.bio,
            pfp: value.pfp,
            banner: value.banner,
            created_at: value.created_at.timestamp(),
            modified_at: value.modified_at.timestamp(),
            admin: value.admin,
            bot: value.bot,
            owner: value.owner.map(|i| Key(i.key().to_owned())),
        }
    }
}

impl From<User> for DBUser {
    fn from(value: User) -> Self {
        Self {
            id: RecordId::from_table_key("user", value.id.0),
            username: value.username,
            display_name: value.display_name,
            bio: value.bio,
            pfp: value.pfp,
            banner: value.banner,
            created_at: chrono::DateTime::from_timestamp(value.created_at, 0)
                .expect("Can't fail")
                .into(),
            modified_at: chrono::DateTime::from_timestamp(value.modified_at, 0)
                .expect("Can't fail")
                .into(),
            admin: value.admin,
            bot: value.bot,
            owner: value.owner.map(|i| RecordId::from_table_key("user", i.0)),
        }
    }
}
