use std::{cell::RefCell, ops::Deref, sync::Arc};

use surrealdb::{engine::remote::ws::Client, Surreal};

use crate::{gql::ApiInfo, models::user::User};

pub struct State {
    pub inner: Arc<InnerState>,
}

pub struct InnerState {
    pub site_name: &'static str,
    pub info: ApiInfo,
    pub redis: rustis::client::Client,
    pub pubsub: rustis::client::Client,
    pub db: Surreal<Client>,
    pub cookie_key: tower_cookies::cookie::Key,
}

impl Deref for State {
    type Target = InnerState;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

#[derive(Default)]
pub struct Auth(pub RefCell<Option<User>>);

// safety: Invarient holds because queries are executed synchronously
unsafe impl Sync for Auth {}
