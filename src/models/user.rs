use super::file::File;
use crate::utils::get_sys_time_secs;
use async_graphql::SimpleObject;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Serialize, Deserialize, Debug)]
pub struct DBUser {
    pub id: Thing,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub pfp: Option<File>,
    pub banner: Option<File>,
    pub created_at: u64,
    pub modified_at: u64,
    pub admin: bool,
    pub bot: bool,
    pub owner: Option<Thing>,
}

impl DBUser {
    /// Creates a new [`DBUser`].
    pub fn new(username: String, bot: bool, owner: Option<Thing>) -> Self {
        let created_at = get_sys_time_secs();
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
            modified_at: created_at,
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
    pub created_at: u64,
    pub modified_at: u64,
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
            created_at: value.created_at,
            modified_at: value.modified_at,
            admin: value.admin,
            bot: value.bot,
            owner: value.owner.map(|i| i.id.to_raw()),
        }
    }
}
