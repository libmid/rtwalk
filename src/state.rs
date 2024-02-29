use std::{ops::Deref, sync::Arc};

use mongodm::mongo;

use crate::gql::ApiInfo;

pub struct State {
    pub inner: Arc<InnerState>,
}

pub struct InnerState {
    pub site_name: &'static str,
    pub info: ApiInfo,
    pub redis: rustis::client::Client,
    pub pubsub: rustis::client::Client,
    pub mongo: mongo::Client,
    pub cookie_key: tower_cookies::cookie::Key,
}

impl Deref for State {
    type Target = InnerState;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}
