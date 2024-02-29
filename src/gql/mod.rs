use self::users::PasswordValidator;
use crate::{config, error::Result, models::user::User, state::State};
use async_graphql::{Context, Object, SimpleObject};
use rustis::{
    client::BatchPreparedCommand,
    commands::{GenericCommands, SetCommands, StringCommands},
};
use tower_cookies::{Cookie, Cookies};

pub mod users;

macro_rules! state {
    ($ctx: expr) => {
        $ctx.data_unchecked::<State>()
    };
}

macro_rules! cookies {
    ($ctx: expr) => {
        $ctx.data_unchecked::<Cookies>()
    };
}

pub struct QueryRoot;

#[derive(SimpleObject, Copy, Clone)]
pub struct ApiInfo {
    pub major: u16,
    pub minor: u16,
    pub bugfix: u16,
    pub rte: &'static str,
    pub vc: &'static str,
}

#[Object]
impl QueryRoot {
    async fn info(&self, ctx: &Context<'_>) -> ApiInfo {
        let state = state!(ctx);
        state.info
    }

    async fn create_user(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(min_length = 4, max_length = 20, regex = r"^[a-z0-9_]+$"))]
        username: String,
        // No sane person has an email longer than that
        #[graphql(validator(max_length = 100, email))] email: String,
        // We assume len(password) < 4 is insecure and > 64 is useless
        #[graphql(validator(
            min_length = 4,
            max_length = 64,
            custom = "PasswordValidator(&username, &email)"
        ))]
        password: String,
    ) -> Result<&str> {
        // On success makes 2 database and 2 redis query.
        // Maimum 2 database and 1 redis query on failure.
        // Also hashing takes place in this step.
        // Also email gets sends here. TODO: Doc if email is sent immediately or pushed to a queue.
        users::push_pending(state!(ctx), username, email, password).await?;
        Ok("Verification code sent to email")
    }

    /// Email verification. You get maximum 4 attempts and code expires 5 minutes after creation.
    async fn verify_user(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(min_length = 4, max_length = 20, regex = r"^[a-z0-9_]+$"))]
        username: String,
        code: u64,
    ) -> Result<User> {
        // Makes 2 database and 3 redis query on success.
        // Makes 3 (max) redis query on fail.
        Ok(users::verify_user(state!(ctx), username, code)
            .await?
            .into())
    }

    async fn login(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(max_length = 100, email))] email: String,
        #[graphql(validator(min_length = 4, max_length = 64,))] password: String,
    ) -> Result<User> {
        // Sends total of 1 database query and 1 redis query on success.
        // 1 database query on failure.
        let state = state!(ctx);
        // Just verifies if credentials are corrent. Nothing to do with cookies and auth.
        // Sends 1 database query every time.
        let user: User = users::login_user(state, email, password).await?.into();
        let cookies = cookies!(ctx);
        let signed_jar = cookies.signed(&state.cookie_key);
        let token = cuid2::cuid();
        signed_jar.add(Cookie::new("session", token.clone()));
        let mut pipeline = state.redis.create_pipeline();
        pipeline
            .set_with_options(
                format!("auth_session:{}", &token),
                serde_json::to_string(&user).expect("Can't fail"),
                rustis::commands::SetCondition::None,
                rustis::commands::SetExpiration::Ex(config::SESSION_EXPIERY_SECONDS),
                false,
            )
            .forget();
        pipeline
            .sadd(format!("session_tracker:{}", &user.id), &token)
            .forget();
        pipeline
            .expire(
                format!("auth_session_tracker:{}", &user.id),
                config::SESSION_EXPIERY_SECONDS,
                rustis::commands::ExpireOption::None,
            )
            .forget();
        pipeline.execute().await?;
        Ok(user)
    }
}
