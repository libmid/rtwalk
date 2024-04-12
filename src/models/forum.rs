use std::borrow::Cow;

use async_graphql::SimpleObject;
use cuid2::cuid;
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};

use super::{file::File, Id};

#[derive(Debug, Serialize, Deserialize)]
pub struct DBForum<'a> {
    pub id: Thing,
    pub owner: Thing,
    pub name: Cow<'a, str>,
    pub display_name: Cow<'a, str>,
    pub description: Option<Cow<'a, str>>,
    pub icon: Option<File>,
    pub banner: Option<File>,
    pub created_at: Datetime,
    pub locked: bool,
}

impl<'a> DBForum<'a> {
    pub fn new(name: &Cow<'a, str>, owner: Id) -> Self {
        Self {
            id: Thing {
                tb: "forum".into(),
                id: cuid().into(),
            },
            owner: Thing {
                tb: "user".into(),
                id: owner.0,
            },
            name: name.clone(),
            display_name: name.clone(),
            description: None,
            icon: None,
            banner: None,
            created_at: Datetime::default(),
            locked: false,
        }
    }
}

#[derive(SimpleObject, Debug)]
pub struct Forum<'a> {
    pub id: Id,
    pub owner_id: Id,
    pub name: Cow<'a, str>,
    pub display_name: Cow<'a, str>,
    pub description: Option<Cow<'a, str>>,
    pub icon: Option<File>,
    pub banner: Option<File>,
    pub created_at: i64,
    pub locked: bool,
}

impl<'a> From<DBForum<'a>> for Forum<'a> {
    fn from(value: DBForum<'a>) -> Self {
        Self {
            id: Id(value.id.id),
            owner_id: Id(value.owner.id),
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
