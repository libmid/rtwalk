use std::borrow::Cow;

use super::{file::File, Id};
use async_graphql::SimpleObject;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};

#[derive(Serialize, Deserialize, Debug)]
pub struct DBUser<'a> {
    pub id: Thing,
    pub username: Cow<'a, str>,
    pub display_name: Cow<'a, str>,
    pub bio: Option<Cow<'a, str>>,
    pub pfp: Option<File>,
    pub banner: Option<File>,
    pub created_at: Datetime,
    pub modified_at: Datetime,
    pub admin: bool,
    pub bot: bool,
    pub owner: Option<Thing>,
}

impl<'a> DBUser<'a> {
    /// Creates a new [`DBUser`].
    pub fn new(username: Cow<'a, str>, bot: bool, owner: Option<Thing>) -> Self {
        let created_at = Datetime::default();
        let modified_at = created_at.clone();
        DBUser {
            id: Thing {
                tb: "user".into(),
                id: cuid2::cuid().into(),
            },
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
pub struct DBUserSecret<'a> {
    pub user: Thing,
    pub email: Cow<'a, str>,
    pub password: Cow<'a, str>,
    pub banned: bool,
}

#[derive(SimpleObject, Serialize, Deserialize, Debug)]
#[graphql(complex)]
pub struct User<'a> {
    pub id: Id,
    pub username: Cow<'a, str>,
    pub display_name: Cow<'a, str>,
    pub bio: Option<Cow<'a, str>>,
    pub pfp: Option<File>,
    pub banner: Option<File>,
    pub created_at: i64,
    pub modified_at: i64,
    pub admin: bool,
    pub bot: bool,
    pub owner: Option<Id>,
}

impl<'r> From<DBUser<'r>> for User<'r> {
    fn from(value: DBUser<'r>) -> Self {
        Self {
            id: Id(value.id.id),
            username: value.username,
            display_name: value.display_name,
            bio: value.bio,
            pfp: value.pfp,
            banner: value.banner,
            created_at: value.created_at.timestamp(),
            modified_at: value.modified_at.timestamp(),
            admin: value.admin,
            bot: value.bot,
            owner: value.owner.map(|i| Id(i.id)),
        }
    }
}

impl<'a> From<User<'a>> for DBUser<'a> {
    fn from(value: User<'a>) -> Self {
        Self {
            id: Thing {
                tb: "user".into(),
                id: value.id.0,
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
                id: i.0,
            }),
        }
    }
}

// impl<'r1, 'r2> ToOwned for User<'r1> {
//     type Owned = User<'r2>;
//     fn to_owned(&self) -> Self::Owned {
//         User {}
//     }
// }
