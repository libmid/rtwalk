use super::file::File;
use async_graphql::SimpleObject;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};

#[derive(Serialize, Deserialize, Debug)]
pub struct DBUser {
    pub id: Thing,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub pfp: Option<File>,
    pub banner: Option<File>,
    pub created_at: Datetime,
    pub modified_at: Datetime,
    pub admin: bool,
    pub bot: bool,
    pub owner: Option<Thing>,
}

impl DBUser {
    /// Creates a new [`DBUser`].
    pub fn new(username: String, bot: bool, owner: Option<Thing>) -> Self {
        let created_at = Datetime::default();
        let modified_at = created_at.clone();
        DBUser {
            id: Thing {
                tb: "user".into(),
                id: cuid2::cuid().into(),
            },
            username: username.clone(),
            display_name: username,
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
    pub user: Thing,
    pub email: String,
    pub password: String,
}

#[derive(SimpleObject, Serialize, Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub pfp: Option<File>,
    pub banner: Option<File>,
    pub created_at: i64,
    pub modified_at: i64,
    pub admin: bool,
    pub bot: bool,
    pub owner: Option<String>,
}

impl From<DBUser> for User {
    fn from(value: DBUser) -> Self {
        Self {
            id: value.id.id.to_raw(),
            username: value.username,
            display_name: value.display_name,
            bio: value.bio,
            pfp: value.pfp,
            banner: value.banner,
            created_at: value.created_at.timestamp(),
            modified_at: value.modified_at.timestamp(),
            admin: value.admin,
            bot: value.bot,
            owner: value.owner.map(|i| i.id.to_raw()),
        }
    }
}

impl From<User> for DBUser {
    fn from(value: User) -> Self {
        Self {
            id: Thing {
                tb: "user".into(),
                id: value.id.into(),
            },
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
            owner: value.owner.map(|i| Thing {
                tb: "user".into(),
                id: i.into(),
            }),
        }
    }
}
