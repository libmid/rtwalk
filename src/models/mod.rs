use std::ops::Deref;

use async_graphql::*;
use serde::{Deserialize, Serialize};
use surrealdb::RecordIdKey;

pub mod file;
pub mod post;
pub mod forum;
pub mod user;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Key(pub RecordIdKey);

impl Deref for Key {
    type Target = RecordIdKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToString for Key {
    fn to_string(&self) -> String {
        self.0.to_string()
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
        Value::String(self.0.to_string())
    }
}
