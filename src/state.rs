use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use opendal::Operator;
use rusty_paseto::generic::{Local, V4};
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
    pub op: Operator,
    pub cookie_key: tower_cookies::cookie::Key,
    pub paseto_key: rusty_paseto::prelude::PasetoSymmetricKey<V4, Local>,
}

impl Deref for State {
    type Target = InnerState;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

#[derive(Default)]
pub struct Auth(pub Mutex<Option<User>>);
