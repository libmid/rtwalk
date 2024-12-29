use std::sync::atomic::{AtomicBool, AtomicU32};

use crate::{
    error::{Result, RtwalkError},
    models::RtEvent,
    state::Auth,
};
use async_graphql::{
    scalar, Context, ErrorExtensions, Guard, MergedObject, Object, ResultExt, SimpleObject,
    Subscription,
};
use async_stream::stream;
use bytes::Buf;
use futures::{Stream, StreamExt};
use rustis::commands::{PubSubCommands, StringCommands};
use serde_json;

pub mod forums;
pub mod posts;
pub mod resolvers;
pub mod users;

macro_rules! state {
    ($ctx: expr) => {{
        use crate::state::State;
        $ctx.data_unchecked::<State>()
    }};
}
use serde::{Deserialize, Serialize};
pub(crate) use state;

macro_rules! cookies {
    ($ctx: expr) => {{
        use tower_cookies::Cookies;
        $ctx.data_unchecked::<Cookies>()
    }};
}
pub(crate) use cookies;

macro_rules! user {
    ($ctx: expr) => {{
        use crate::state::Auth;
        $ctx.data_unchecked::<Auth>()
            .0
            .lock()
            .unwrap()
            .take()
            .unwrap()
    }};
}
pub(crate) use user;

#[derive(Default)]
pub struct QueryRoot;

#[derive(SimpleObject, Copy, Clone)]
pub struct ApiInfo {
    pub major: u16,
    pub minor: u16,
    pub bugfix: u16,
    pub rte: &'static str,
    pub vc: &'static str,
}

#[derive(Eq, PartialEq, Copy, Clone)]
enum Role {
    Bot,             // Only bot
    UnAuthenticated, // Only unauthenticated
    Authenticated,   // Any authenticated user
    Human,           // Only human
    Admin,           // Only admin
}

impl Guard for Role {
    async fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let state = state!(ctx);
        let cookeis = ctx.data_unchecked::<tower_cookies::Cookies>();
        let jar = cookeis.signed(&state.cookie_key);
        let token = jar.get("session");
        if let Some(token) = token {
            let user: Option<String> = state
                .redis
                .get(format!("auth_session:{}", token.value()))
                .await
                .map_err(|e| RtwalkError::RedisError(e))
                .extend_err(|_, _| {})?;
            if let Some(user) = user {
                let user: crate::models::user::User = serde_json::from_str(&user)
                    .map_err(|e| {
                        RtwalkError::ImpossibleError(
                            "Deserialization of User can't fail",
                            Some(e.into()),
                        )
                    })
                    .extend_err(|_, _| {})?;
                let permitted = match self {
                    Self::Admin => user.admin,
                    Self::Bot => user.bot,
                    Self::Human => !user.bot,
                    Self::Authenticated => true,
                    Self::UnAuthenticated => unreachable!(),
                };
                if permitted {
                    *ctx.data_unchecked::<Auth>().0.lock().unwrap() = Some(user);
                    return Ok(());
                }
            }
        }
        if *self == Self::UnAuthenticated {
            return Ok(());
        }
        return Err(crate::error::RtwalkError::UnauthenticatedRequest
            .extend()
            .into());
    }
}

#[derive(SimpleObject)]
#[graphql(complex, serial)]
pub struct Page {
    /// IMPORTANT: pageInfo must always be placed after your query
    page_info: PageInfo,
}

#[derive(SimpleObject, Default)]
pub struct PageInfo {
    #[graphql(skip)]
    pub page: u32,
    #[graphql(skip)]
    pub per_page: u32,
    #[graphql(skip)]
    needs_page_info: bool,
    total: TotalCount,
    has_next_page: HasNextPage,
}

#[derive(Serialize, Deserialize, Default)]
pub struct TotalCount(pub AtomicU32);
#[derive(Serialize, Deserialize, Default)]
pub struct HasNextPage(pub AtomicBool);

scalar!(TotalCount, "Int");
scalar!(HasNextPage, "Boolean");

#[Object]
impl QueryRoot {
    async fn info(&self, ctx: &Context<'_>) -> ApiInfo {
        let state = state!(ctx);
        state.info
    }

    #[graphql(name = "Page")]
    async fn page(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 1)] page: u32,
        #[graphql(default = 20)] per_page: u32,
    ) -> Result<Page> {
        let mut needs_page_info = false;

        for field in ctx.look_ahead().selection_fields() {
            if field.name() == "Page" {
                let mut set_count = 0;
                for set in field.selection_set() {
                    if set.name() != "pageInfo" {
                        set_count += 1;
                    } else {
                        needs_page_info = true;
                    }
                }
                if set_count > 1 {
                    return Err(RtwalkError::MultiplePageField).extend_err(|_, _| {});
                }
            }
        }
        Ok(Page {
            page_info: PageInfo {
                page,
                per_page,
                needs_page_info,
                ..Default::default()
            },
        })
    }
}

pub struct Subscription;

#[Subscription]
impl Subscription {
    async fn rte(
        &self,
        ctx: &Context<'_>,
        post_create: bool,
        post_update: bool,
    ) -> async_graphql::Result<impl Stream<Item = RtEvent>> {
        let state = state!(ctx);

        let mut channels = vec![];
        if post_create {
            channels.push("rte-post-create");
        }
        if post_update {
            channels.push("rte-post-update");
        }

        let mut sub_stream = state
            .pubsub
            .ssubscribe(channels)
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;

        Ok(stream! {
            while let Some(maybe_sub_msg) = sub_stream.next().await {
                if let Ok(sub_msg) = maybe_sub_msg {
                    let event: RtEvent = serde_json::from_reader(sub_msg.payload.reader()).expect("Payload must be valid");

                    yield event;
                }
                // TODO: Handle this error
            }
        })
    }
}

#[derive(MergedObject, Default)]
#[graphql(name = "Query")]
pub struct MergedQueryRoot(
    QueryRoot,
    resolvers::users::UserQueryRoot,
    resolvers::forums::ForumQueryRoot,
);

#[derive(MergedObject, Default)]
#[graphql(name = "Mutation")]
pub struct MergedMutationRoot(
    resolvers::users::UserMutationRoot,
    resolvers::forums::ForumMutationRoot,
    resolvers::posts::PostMutationRoot,
);
