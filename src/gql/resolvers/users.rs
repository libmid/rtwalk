use async_graphql::SimpleObject;
use async_graphql::{ComplexObject, Context, Object, ResultExt};
use surrealdb::RecordId;

use crate::models::Key;
use crate::{
    config,
    error::RtwalkError,
    models::file::{File, FileOps},
};
use async_graphql::{Guard, MaybeUndefined, OneofObject, Upload};
use cuid2::cuid;
use rustis::{
    client::BatchPreparedCommand,
    commands::{GenericCommands, SetCommands, StringCommands},
};
use tower_cookies::cookie::time::Duration;
use tower_cookies::Cookie;

use super::super::{cookies, state, user, users, users::PasswordValidator, Role};
use crate::models::user::{DBUser, User};

#[ComplexObject]
impl User {
    #[graphql(guard = Role::Authenticated)]
    async fn bots(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<User>> {
        let state = state!(ctx);
        let user = user!(ctx);

        if user.admin || user.id == self.id {
            let mut res = state
                .db
                .query("SELECT * FROM user WHERE owner = $owner")
                .bind(("owner", RecordId::from_table_key("user", self.id.clone().0)))
                .await
                .map_err(|e| RtwalkError::from(e))
                .extend_err(|_, _| {})?;

            let bots: Vec<DBUser> = res
                .take(0)
                .map_err(|e| RtwalkError::from(e))
                .extend_err(|_, _| {})?;

            return Ok(bots.into_iter().map(|x| x.into()).collect());
        }
        Err(RtwalkError::UnauhorizedRequest)
            .extend_err(|_, _| {})
            .into()
    }
}

#[derive(Default)]
pub struct UserQueryRoot;

#[derive(OneofObject)]
pub enum UserSelectCriteria {
    Id(String),
    Username(String),
}

#[derive(OneofObject)]
pub enum MultipleUserSelectCriteria {
    Ids(Vec<String>),
    Usernames(Vec<String>),
    Search(String),
}

#[Object]
impl UserQueryRoot {
    #[graphql(guard = Role::Authenticated)]
    async fn me(&self, ctx: &Context<'_>) -> async_graphql::Result<User> {
        Ok(user!(ctx))
    }

    async fn user<'r>(
        &self,
        ctx: &Context<'r>,
        criteria: UserSelectCriteria,
    ) -> async_graphql::Result<Option<User>> {
        let state = state!(ctx);

        let user = users::fetch_user(state, criteria)
            .await
            .extend_err(|_, _| {})?;

        Ok(user.map(|x| x.into()))
    }
}

#[derive(SimpleObject)]
struct Bot {
    token: String,
    #[graphql(flatten)]
    bot: User,
}

#[derive(Default)]
pub struct UserMutationRoot;

#[Object]
impl UserMutationRoot {
    /// Account rgistration process starts here. Sends a code to your email.
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
            custom = PasswordValidator(&username, &email)
        ))]
        password: String,
    ) -> async_graphql::Result<&str> {
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
    async fn verify_user<'r>(
        &self,
        ctx: &Context<'r>,
        #[graphql(validator(min_length = 4, max_length = 20, regex = r"^[a-z0-9_]+$"))]
        username: String,
        code: u64,
    ) -> async_graphql::Result<User> {
        // Makes 1 database and 3 redis query on success.
        // Makes 3 (max) redis query on failure.
        Ok(users::verify_user(state!(ctx), username, code)
            .await
            .extend_err(|_, _| {})?
            .into())
    }

    #[graphql(guard = "Role::UnAuthenticated")]
    async fn login<'r>(
        &self,
        ctx: &Context<'r>,
        #[graphql(validator(max_length = 100))] email: String,
        #[graphql(validator(min_length = 4, max_length = 64,))] password: String,
    ) -> async_graphql::Result<User> {
        // Sends total of 1 database query and 1 redis query on success.
        // 1 database query on failure.

        let state = state!(ctx);
        // Just verifies if credentials are corrent. Nothing to do with cookies and auth.
        // Sends 1 database query every time.
        let user: User = users::login_user(state, email, password)
            .await
            .extend_err(|_, _| {})?
            .into();

        let token = cuid2::cuid();
        let mut pipeline = state.redis.create_pipeline();
        pipeline
            .set_with_options(
                format!("auth_session:{}", &token),
                serde_json::to_string(&user)
                    .map_err(|e| {
                        RtwalkError::ImpossibleError(
                            "Serialization of User can't fail",
                            Some(e.into()),
                        )
                    })
                    .extend_err(|_, _| {})?,
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
            .sadd(
                format!("auth_session_tracker:{}", user.id.to_string()),
                &token,
            )
            .forget();
        pipeline
            .expire(
                format!("auth_session_tracker:{}", user.id.to_string()),
                if user.bot {
                    config::BOT_SESSION_EXPIERY
                } else {
                    config::SESSION_EXPIERY_SECONDS
                },
                rustis::commands::ExpireOption::None,
            )
            .forget();
        pipeline
            .execute::<()>()
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;
        let cookies = cookies!(ctx);
        let signed_jar = cookies.signed(&state.cookie_key);
        let mut cookie = Cookie::new("session", token);
        cookie.set_max_age(Duration::seconds(if user.bot {
            config::BOT_SESSION_EXPIERY as i64
        } else {
            config::SESSION_EXPIERY_SECONDS as i64
        }));
        // TODO: Set secure
        // cookie.set_secure(true);
        signed_jar.add(cookie);

        Ok(user)
    }

    // Logout current user session
    #[graphql(guard = "Role::Authenticated")]
    async fn logout(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
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
            .srem(
                format!("auth_session_tracker:{}", user.id.to_string()),
                token.value(),
            )
            .forget();
        pipeline
            .execute::<()>()
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;

        jar.remove(Cookie::new("session", ""));
        Ok(true)
    }

    /// Logs out all active/inactive sessions on all devices
    #[graphql(guard = "Role::Authenticated")]
    async fn logout_all(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        // Sends 2 redis queries.
        let user = user!(ctx);
        let state = state!(ctx);
        let cookies = cookies!(ctx);

        let sessions: Vec<String> = state
            .redis
            .smembers(format!("auth_session_tracker:{}", user.id.to_string()))
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;
        let mut pipeline = state.redis.create_pipeline();
        for session in sessions {
            pipeline.del(format!("auth_session:{}", session)).forget();
        }
        pipeline
            .del(format!("auth_session_tracker:{}", user.id.to_string()))
            .forget();
        pipeline
            .execute::<()>()
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;

        let jar = cookies.signed(&state.cookie_key);
        jar.remove(Cookie::new("session", ""));

        Ok(true)
    }

    /// Only humans can create bots.
    /// Returns bot credentials.
    #[graphql(guard = "Role::Human")]
    async fn create_bot<'r>(
        &self,
        ctx: &Context<'r>,
        username: String,
    ) -> async_graphql::Result<Bot> {
        let user = user!(ctx);
        // 2 database and 1 redis query on sucess
        let (token, bot) = users::create_bot(state!(ctx), user.id, username)
            .await
            .extend_err(|_, _| {})?;
        Ok(Bot {
            token,
            bot: bot.into(),
        })
    }

    /// Only non-bot accounts who own the bot can do this.
    #[graphql(guard = "Role::Human")]
    async fn login_as_bot<'r>(
        &self,
        ctx: &Context<'r>,
        bot_id: Key,
    ) -> async_graphql::Result<User> {
        let user = user!(ctx);
        let state = state!(ctx);
        let cookies = cookies!(ctx);
        let bot: User = users::verify_bot_belongs_to_user(state, &user.id, &bot_id)
            .await
            .extend_err(|_, _| {})?
            .into();
        {
            // logout user
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
                .srem(
                    format!("auth_session_tracker:{}", user.id.to_string()),
                    token.value(),
                )
                .forget();
            pipeline
                .execute::<()>()
                .await
                .map_err(|e| RtwalkError::RedisError(e))
                .extend_err(|_, _| {})?;

            jar.remove(Cookie::new("session", ""));
        }
        {
            // login bot
            let token = cuid2::cuid();
            let mut pipeline = state.redis.create_pipeline();
            pipeline
                .set_with_options(
                    format!("auth_session:{}", &token),
                    serde_json::to_string(&bot)
                        .map_err(|e| {
                            RtwalkError::ImpossibleError(
                                "Serialization of bot can't fail",
                                Some(e.into()),
                            )
                        })
                        .extend_err(|_, _| {})?,
                    rustis::commands::SetCondition::None,
                    rustis::commands::SetExpiration::Ex(if bot.bot {
                        config::BOT_SESSION_EXPIERY
                    } else {
                        config::SESSION_EXPIERY_SECONDS
                    }),
                    false,
                )
                .forget();
            pipeline
                .sadd(
                    format!("auth_session_tracker:{}", bot.id.to_string()),
                    &token,
                )
                .forget();
            pipeline
                .expire(
                    format!("auth_session_tracker:{}", bot.id.to_string()),
                    if bot.bot {
                        config::BOT_SESSION_EXPIERY
                    } else {
                        config::SESSION_EXPIERY_SECONDS
                    },
                    rustis::commands::ExpireOption::None,
                )
                .forget();
            pipeline
                .execute::<()>()
                .await
                .map_err(|e| RtwalkError::RedisError(e))
                .extend_err(|_, _| {})?;
            let signed_jar = cookies.signed(&state.cookie_key);
            let mut cookie = Cookie::new("session", token);
            cookie.set_max_age(Duration::seconds(if bot.bot {
                config::BOT_SESSION_EXPIERY as i64
            } else {
                config::SESSION_EXPIERY_SECONDS as i64
            }));
            cookie.set_secure(true);
            signed_jar.add(cookie);

            Ok(bot)
        }
    }

    /// Only non-bot accounts who own the bot can do this.
    #[graphql(guard = "Role::Human")]
    async fn logout_all_bot(&self, ctx: &Context<'_>, bot_id: Key) -> async_graphql::Result<bool> {
        let user = user!(ctx);
        let state = state!(ctx);

        let bot: User = users::verify_bot_belongs_to_user(state, &user.id, &bot_id)
            .await
            .extend_err(|_, _| {})?
            .into();

        let sessions: Vec<String> = state
            .redis
            .smembers(format!("auth_session_tracker:{}", bot.id.to_string()))
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;
        let mut pipeline = state.redis.create_pipeline();
        for session in sessions {
            pipeline.del(format!("auth_session:{}", session)).forget();
        }
        pipeline
            .del(format!("auth_session_tracker:{}", bot.id.to_string()))
            .forget();
        pipeline
            .execute::<()>()
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;

        Ok(true)
    }

    /// Only non-bot accounts who own the bot can do this.
    #[graphql(guard = "Role::Human")]
    async fn reset_bot_token<'r>(
        &self,
        ctx: &Context<'r>,
        bot_id: Key,
    ) -> async_graphql::Result<Bot> {
        let user = user!(ctx);
        let state = state!(ctx);

        let bot: User = users::verify_bot_belongs_to_user(state, &user.id, &bot_id)
            .await
            .extend_err(|_, _| {})?
            .into();

        // reset token
        let token = users::reset_bot_password(state, &bot.id)
            .await
            .extend_err(|_, _| {})?;

        // logout the bot
        let sessions: Vec<String> = state
            .redis
            .smembers(format!("auth_session_tracker:{}", bot.id.to_string()))
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;
        let mut pipeline = state.redis.create_pipeline();
        for session in sessions {
            pipeline.del(format!("auth_session:{}", session)).forget();
        }
        pipeline
            .del(format!("auth_session_tracker:{}", bot.id.to_string()))
            .forget();
        pipeline
            .execute::<()>()
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;

        Ok(Bot { token, bot })
    }

    async fn reset_password(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(max_length = 100, email))] email: String,
    ) -> async_graphql::Result<bool> {
        users::reset_password(state!(ctx), &email)
            .await
            .extend_err(|_, _| {})?;
        Ok(true)
    }

    async fn verify_password(
        &self,
        ctx: &Context<'_>,
        token: String,
        #[graphql(validator(
            min_length = 4,
            max_length = 64,
            custom = r#"PasswordValidator("", "")"#
        ))]
        new_password: String,
    ) -> async_graphql::Result<bool> {
        users::set_new_password(state!(ctx), &token, new_password)
            .await
            .extend_err(|_, _| {})?;
        Ok(true)
    }

    #[graphql(guard = "Role::Human")]
    async fn change_email(
        &self,
        _ctx: &Context<'_>,
        _email: String,
    ) -> async_graphql::Result<bool> {
        // TODO:
        todo!()
    }

    #[graphql(guard = "Role::Authenticated")]
    async fn update_user<'r>(
        &self,
        ctx: &Context<'r>,
        #[graphql(validator(min_length = 4, max_length = 20, regex = r"^[a-z0-9_]+$"))]
        username: Option<String>,
        display_name: Option<String>,
        bio: MaybeUndefined<String>,
        pfp: MaybeUndefined<Upload>,
        banner: MaybeUndefined<Upload>,
    ) -> async_graphql::Result<User> {
        let mut user = user!(ctx);
        let state = state!(ctx);

        if let Some(username) = username {
            user.username = username.into();
        }
        if let Some(display_name) = display_name {
            user.display_name = display_name.into();
        }
        if bio.is_null() {
            user.bio = None;
        } else if let MaybeUndefined::Value(v) = bio {
            user.bio = Some(v.into());
        }
        if pfp.is_null() {
            user.pfp.delete(&state.op).await.extend_err(|_, _| {})?;
            user.pfp = None;
        } else if let MaybeUndefined::Value(v) = pfp {
            let mut upload_value = v.value(&ctx)?;
            if upload_value.size()? > config::MAX_UPLOAD_SIZE {
                return Err(RtwalkError::MaxUploadSizeExceeded).extend_err(|_, _| {})?;
            }

            user.pfp.delete(&state.op).await.extend_err(|_, _| {})?;

            let pfp_file = File {
                loc: format!(
                    "{}/{}-{}",
                    user.id.to_string(),
                    cuid(),
                    upload_value.filename
                ),
            };
            pfp_file
                .save(&state.op, &mut upload_value)
                .await
                .extend_err(|_, _| {})?;
            user.pfp = Some(pfp_file);
        }
        if banner.is_null() {
            user.banner.delete(&state.op).await.extend_err(|_, _| {})?;

            user.banner = None;
        } else if let MaybeUndefined::Value(v) = banner {
            let mut upload_value = v.value(&ctx)?;
            if upload_value.size()? > config::MAX_UPLOAD_SIZE {
                return Err(RtwalkError::MaxUploadSizeExceeded).extend_err(|_, _| {})?;
            }

            user.banner.delete(&state.op).await.extend_err(|_, _| {})?;

            let banner_file = File {
                loc: format!(
                    "{}/{}-{}",
                    user.id.to_string(),
                    cuid(),
                    upload_value.filename
                ),
            };
            banner_file
                .save(&state.op, &mut upload_value)
                .await
                .extend_err(|_, _| {})?;
            user.banner = Some(banner_file);
        }

        let user: User = users::update_user(state, user)
            .await
            .extend_err(|_, _| {})?
            .into();

        // TODO: Modify sessions instead of logging everyone out
        Role::Authenticated.check(ctx).await.extend_err(|_, _| {})?;
        self.logout_all(ctx).await.extend_err(|_, _| {})?;

        let token = cuid2::cuid();
        let mut pipeline = state.redis.create_pipeline();
        pipeline
            .set_with_options(
                format!("auth_session:{}", &token),
                serde_json::to_string(&user)
                    .map_err(|e| {
                        RtwalkError::ImpossibleError(
                            "Serialization of User can't fail",
                            Some(e.into()),
                        )
                    })
                    .extend_err(|_, _| {})?,
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
            .sadd(
                format!("auth_session_tracker:{}", user.id.to_string()),
                &token,
            )
            .forget();
        pipeline
            .expire(
                format!("auth_session_tracker:{}", user.id.to_string()),
                if user.bot {
                    config::BOT_SESSION_EXPIERY
                } else {
                    config::SESSION_EXPIERY_SECONDS
                },
                rustis::commands::ExpireOption::None,
            )
            .forget();
        pipeline
            .execute::<()>()
            .await
            .map_err(|e| RtwalkError::RedisError(e))
            .extend_err(|_, _| {})?;
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

    #[graphql(guard = Role::Admin, visible = false)]
    async fn ban_user(
        &self,
        _ctx: &Context<'_>,
        _to_ban_id: String,
    ) -> async_graphql::Result<bool> {
        // TODO: Log the ban and who did it
        todo!()
    }

    #[graphql(guard = Role::Authenticated)]
    async fn delete_file(&self, ctx: &Context<'_>, loc: String) -> async_graphql::Result<bool> {
        let state = state!(ctx);
        let user = user!(ctx);

        if let Some(user_id) = loc.split('/').next() {
            if user_id == &user.id.to_string() || user.admin {
                File { loc }.delete(&state.op).await.extend_err(|_, _| {})?;
            }
        }

        Ok(true)
    }
}
