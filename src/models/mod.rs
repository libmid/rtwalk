use std::ops::Deref;

use async_graphql::*;
use comment::Comment;
use post::Post;
use serde::{de::Visitor, Deserialize, Serialize};
use surrealdb::types::{RecordIdKey, SurrealValue};

pub mod comment;
pub mod file;
pub mod forum;
pub mod post;
pub mod user;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Key(pub RecordIdKey);
struct KeyVisitor;

impl Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Visitor<'de> for KeyVisitor {
    type Value = Key;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid string representing a RecordIdKey")
    }

    fn visit_str<E>(self, v: &str) -> std::prelude::v1::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let inner = RecordIdKey::from(v.to_string());
        Ok(Key(inner))
    }

    fn visit_string<E>(self, v: String) -> std::prelude::v1::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Key(RecordIdKey::from(v)))
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(KeyVisitor)
    }
}

impl Deref for Key {
    type Target = RecordIdKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToString for Key {
    fn to_string(&self) -> String {
        self.0
            .clone()
            .into_value()
            .into_string()
            .expect("key to string always possible")
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Self(RecordIdKey::from(value))
    }
}

#[Scalar]
impl ScalarType for Key {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = value {
            Ok(Key(value.parse::<String>()?.into()))
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

#[derive(SimpleObject, Deserialize, Serialize, Clone)]
pub struct PostCreateEvent {
    pub data: Post,
}

#[derive(SimpleObject, Deserialize, Serialize, Clone)]
pub struct PostEditEvent {
    pub original: Post,
    pub new: Post,
}

#[derive(SimpleObject, Deserialize, Serialize, Clone)]
pub struct CommentCreateEvent {
    pub data: Comment,
}

#[derive(SimpleObject, Deserialize, Serialize, Clone)]
pub struct CommentEditEvent {
    pub original: Comment,
    pub new: Comment,
}

#[derive(Union, Deserialize, Serialize, Clone)]
pub enum RtEventData {
    PostCreate(PostCreateEvent),
    PostEdit(PostEditEvent),
    CommentCreate(CommentCreateEvent),
    CommentEdit(CommentEditEvent),
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Enum)]
pub enum RtEventType {
    PostCreate,
    PostEdit,
    CommentCreate,
    CommentEdit,
}

#[derive(SimpleObject, Serialize, Deserialize, Clone)]
pub struct RtEvent {
    pub ty: RtEventType,
    #[graphql(flatten)]
    #[serde[flatten]]
    pub event_data: RtEventData,
}
