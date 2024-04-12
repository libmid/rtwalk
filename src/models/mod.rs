use std::ops::Deref;

use async_graphql::*;
use serde::{Deserialize, Serialize};

pub mod file;
pub mod forum;
pub mod user;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Id(pub surrealdb::sql::Id);

impl Deref for Id {
    type Target = surrealdb::sql::Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToString for Id {
    fn to_string(&self) -> String {
        self.0.to_raw()
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        match &self.0 {
            surrealdb::sql::Id::String(s) => &s,
            _ => unreachable!(),
        }
    }
}

impl From<String> for Id {
    fn from(value: String) -> Self {
        Self(surrealdb::sql::Id::from(value))
    }
}

#[Scalar]
impl ScalarType for Id {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = value {
            Ok(Id(value.parse::<String>()?.into()))
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}
