use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use surrealdb::Surreal;

use crate::{gql::ApiInfo, models::user::User};

pub struct State<S> {
    pub inner: Arc<InnerState<S>>,
}

pub struct InnerState<S> {
    pub site_name: &'static str,
    pub info: ApiInfo,
    pub redis: rustis::client::Client,
    pub pubsub: rustis::client::Client,
    pub db: Surreal<S>,
    pub cookie_key: tower_cookies::cookie::Key,
}

impl<S> Deref for State<S> {
    type Target = InnerState<S>;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

#[derive(Default)]
pub struct Auth(pub Mutex<Option<User>>);
