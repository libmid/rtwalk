use self::users::{create_bot, PasswordValidator};
use crate::{
    config,
    error::{Result, RtwalkError},
    models::user::User,
    state::{Auth, State},
};
use async_graphql::{Context, Guard, Object, ResultExt, SimpleObject};
use rustis::{
    client::BatchPreparedCommand,
    commands::{GenericCommands, SetCommands, StringCommands},
};
use tower_cookies::cookie::time::Duration;
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

macro_rules! user {
    ($ctx: expr) => {
        $ctx.data_unchecked::<Auth>().0.take().unwrap()
    };
}

pub struct QueryRoot;
pub struct MutationRoot;

#[derive(SimpleObject, Copy, Clone)]
pub struct ApiInfo {
    pub major: u16,
    pub minor: u16,
    pub bugfix: u16,
    pub rte: &'static str,
    pub vc: &'static str,
}

#[derive(SimpleObject)]
struct Bot {
    token: String,
    bot: User,
}

#[derive(Eq, PartialEq, Copy, Clone)]
enum Role {
    Bot,           // Only bot
    Authenticated, // Any authenticated user
    Human,         // Only human
    Admin,         // Only admin
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
                .await?;
            if let Some(user) = user {
                let user: crate::models::user::User = serde_json::from_str(&user).map_err(|e| {
                    RtwalkError::ImpossibleError(
                        "Deserialization of User can't fail",
                        Some(e.into()),
                    )
                })?;
                let permitted = match self {
                    Self::Admin => user.admin,
                    Self::Bot => user.bot,
                    Self::Human => !user.bot,
                    Self::Authenticated => true,
                };
                if permitted {
                    *ctx.data_unchecked::<Auth>().0.borrow_mut() = Some(user);
                    return Ok(());
                }
            }
        }
        return Err(crate::error::RtwalkError::UnauthenticatedRequest.into());
    }
}

#[Object]
impl QueryRoot {
    async fn info(&self, ctx: &Context<'_>) -> ApiInfo {
        let state = state!(ctx);
        state.info
    }

    #[graphql(guard = "Role::Authenticated")]
    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        Ok(user!(ctx))
    }
}

#[Object]
impl MutationRoot {
    /// Account rgistration process starts here
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
        // On success makes 1 database and 2 redis query.
        // Maximum 1 database and 1 redis query on failure.
        // Also hashing takes place in this step. Its normal for latency to be > 1s.
        // Also email gets sends here. TODO: Doc if email is sent immediately or pushed to a queue.
        users::push_pending(state!(ctx), username, email, password)
            .await
            .extend_err(|_, _| {})?;
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
        // Makes 1 database and 3 redis query on success.
        // Makes 3 (max) redis query on failure.
        Ok(users::verify_user(state!(ctx), username, code)
            .await
            .extend_err(|_, _| {})?
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

        let token = cuid2::cuid();
        let mut pipeline = state.redis.create_pipeline();
        pipeline
            .set_with_options(
                format!("auth_session:{}", &token),
                serde_json::to_string(&user).map_err(|e| {
                    RtwalkError::ImpossibleError("Serialization of User can't fail", Some(e.into()))
                })?,
                rustis::commands::SetCondition::None,
                rustis::commands::SetExpiration::Ex(if user.bot {
                    config::BOT_SESSION_EXPIERY
                } else {
                    config::SESSION_EXPIERY_SECONDS
                }),
                false,
            )
            .forget();
        pipeline
            .sadd(format!("auth_session_tracker:{}", &user.id), &token)
            .forget();
        pipeline
            .expire(
                format!("auth_session_tracker:{}", &user.id),
                if user.bot {
                    config::BOT_SESSION_EXPIERY
                } else {
                    config::SESSION_EXPIERY_SECONDS
                },
                rustis::commands::ExpireOption::None,
            )
            .forget();
        pipeline.execute().await?;
        let cookies = cookies!(ctx);
        let signed_jar = cookies.signed(&state.cookie_key);
        let mut cookie = Cookie::new("session", token);
        cookie.set_max_age(Duration::seconds(if user.bot {
            config::BOT_SESSION_EXPIERY as i64
        } else {
            config::SESSION_EXPIERY_SECONDS as i64
        }));
        cookie.set_secure(true);
        signed_jar.add(cookie);

        Ok(user)
    }

    // Logout current user session
    #[graphql(guard = "Role::Authenticated")]
    async fn logout(&self, ctx: &Context<'_>) -> Result<bool> {
        let state = state!(ctx);
        let user = user!(ctx);
        let cookies = cookies!(ctx);

        let jar = cookies.signed(&state.cookie_key);
        let token = jar
            .get("session")
            .ok_or(RtwalkError::ImpossibleError(
                "Guard already proves invarient that session token exists",
                None,
            ))
            .extend_err(|_, _| {})?;

        let mut pipeline = state.redis.create_pipeline();
        pipeline
            .del(format!("auth_session:{}", token.value()))
            .forget();
        pipeline
            .srem(format!("auth_session_tracker:{}", &user.id), token.value())
            .forget();
        pipeline.execute().await?;

        jar.remove(Cookie::new("session", ""));
        Ok(true)
    }

    /// Logs out all active/inactive sessions on all devices
    #[graphql(guard = "Role::Authenticated")]
    async fn logout_all(&self, ctx: &Context<'_>) -> Result<bool> {
        // Sends 2 redis queries.
        let user = user!(ctx);
        let state = state!(ctx);

        let sessions: Vec<String> = state
            .redis
            .smembers(format!("auth_session_tracker:{}", &user.id))
            .await?;
        let mut pipeline = state.redis.create_pipeline();
        for session in sessions {
            pipeline.del(format!("auth_session:{}", session)).forget();
        }
        pipeline
            .del(format!("auth_session_tracker:{}", &user.id))
            .forget();
        pipeline.execute().await?;
        Ok(true)
    }

    /// Only humans can create bots
    /// Returns bot credentials
    #[graphql(guard = "Role::Human")]
    async fn create_bot(&self, ctx: &Context<'_>, username: String) -> Result<Bot> {
        let user = user!(ctx);
        // 2 database and 1 redis query on sucess
        let (token, bot) = create_bot(state!(ctx), user.id, username)
            .await
            .extend_err(|_, _| {})?;
        Ok(Bot {
            token,
            bot: bot.into(),
        })
    }

    // #[graphql(guard = "Role::Human")]
    async fn login_as_bot(&self, ctx: &Context<'_>, bot_id: String) -> Result<User> {
        {
            let state = state!(ctx);
            verify_bot_belongs_to_user(state)
                .await
                .extend_err(|_, _| {})?;
        }
        todo!()
    }
}
