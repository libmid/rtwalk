use crate::{
    error::{Result, RtwalkError},
    models::user::User,
    state::Auth,
};
use async_graphql::{
    Context, ErrorExtensions, Guard, MergedObject, Object, ResultExt, SimpleObject,
};
use rustis::commands::StringCommands;

pub mod resolvers;
pub mod users;

macro_rules! state {
    ($ctx: expr) => {{
        use crate::state::State;
        $ctx.data_unchecked::<State>()
    }};
}
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

#[Object]
impl QueryRoot {
    async fn info(&self, ctx: &Context<'_>) -> ApiInfo {
        let state = state!(ctx);
        state.info
    }
}

#[derive(MergedObject, Default)]
#[graphql(name = "Query")]
pub struct MergedQueryRoot(QueryRoot, resolvers::users::UserQueryRoot);

#[derive(MergedObject, Default)]
#[graphql(name = "Mutation")]
pub struct MergedMutationRoot(resolvers::users::UserMutationRoot);
