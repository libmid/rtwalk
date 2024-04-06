use crate::config;
use crate::models::user::User;
use crate::{
    error::RtwalkError,
    models::user::{DBUser, DBUserSecret},
    state::State,
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_graphql::CustomValidator;
use cuid2::cuid;
use rand::Rng;

use rustis::commands::GenericCommands;
use rustis::{
    client::BatchPreparedCommand,
    commands::{SetCondition, SetExpiration, StringCommands},
};
use rusty_paseto::prelude::*;
use surrealdb::sql::{Datetime, Thing};
use zxcvbn::zxcvbn;

use super::resolvers::users::UserSelectCriteria;

pub struct PasswordValidator<'a>(pub &'a str, pub &'a str);

impl<'a> CustomValidator<String> for PasswordValidator<'a> {
    fn check(
        &self,
        value: &String,
    ) -> std::result::Result<(), async_graphql::InputValueError<String>> {
        let entropy = zxcvbn(value, &[self.0, self.1]).map_err(|err| {
            // NOTE: Maybe make this a part of the error mod?
            async_graphql::InputValueError::from(err).with_extension("tp", "INVALID_PASSWORD")
        })?;
        if entropy.score() < config::MIN_PASSWORD_SCORE {
            // NOTE: Same as above
            return Err(async_graphql::InputValueError::from(format!(
                "Password too weak [{}/{}]",
                entropy.score(),
                4
            ))
            .with_extension("tp", "WEAK_PASSWORD"));
        }
        Ok(())
    }
}

pub async fn push_pending(
    state: &State,
    username: String,
    email: String,
    password: String,
) -> Result<(), RtwalkError> {
    // Assumes data is already validated.
    // First make sure username is unique
    let mut exists = state
        .db
        .query("SELECT 1 FROM user WHERE username = $username")
        .query("SELECT 1 FROM user_secret WHERE email = $email")
        .bind(("username", &username))
        .bind(("email", &email))
        .await?;
    let username_exists: Option<u64> = exists.take((0, "1"))?;
    if username_exists.is_some() {
        return Err(RtwalkError::UsernameAlreadyExists);
    }
    // Make sure no one is trying to verify with the same suername
    let user: Option<String> = state.redis.get(format!("pending:{}", &username)).await?;
    if user.is_some() {
        return Err(RtwalkError::UsernameAlreadyExists);
    }

    // Check if user with same email already exists.
    let email_exists: Option<u64> = exists.take((1, "1"))?;
    if email_exists.is_some() {
        // Silently drop.
        return Ok(());
    }
    // Hash password
    let password_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| RtwalkError::InternalError(e.into()))
            .map(|h| h.to_string())
    })
    .await
    .map_err(|e| RtwalkError::InternalError(e.into()))??;
    // We have verified and confirmed user is valid, generate verification code.
    let code = rand::thread_rng().gen_range(10000..=99999);
    // Send email to the user
    // let _template = EmailVerify {
    //     username: &username,
    //     code,
    //     site_name: state.site_name,
    // }
    // .render_once()
    // .expect("Can't fail");
    // TODO: Actually send the mail, just printing for now
    // WARNING: Dont forget this ^
    dbg!("Email verification code", code);
    // Construct user and secret.
    let (pending_user_key, tries_remaining_key, secret_key, verification_code_key) = (
        format!("pending:{}", &username),
        format!("remaining_tries:{}", &username),
        format!("pending_secret:{}", &username),
        format!("verification_code:{}", &username),
    );
    let user = DBUser::new(username, false, None);
    let secret = DBUserSecret {
        user: Thing {
            tb: "user".into(),
            id: user.id.clone().into(),
        },
        email,
        password: password_hash,
    };
    // Push the state into redis with a ttl.
    let mut pipeline = state.redis.create_pipeline();
    pipeline
        .set_with_options(
            pending_user_key,
            serde_json::to_string(&user).map_err(|e| {
                RtwalkError::ImpossibleError("Serialization of DBUser can't fail", Some(e.into()))
            })?,
            SetCondition::None,
            SetExpiration::Ex(config::VERIFICATION_CODE_EXPIERY_SECONDS),
            false,
        )
        .forget();
    pipeline
        .set_with_options(
            secret_key,
            serde_json::to_string(&secret).map_err(|e| {
                RtwalkError::ImpossibleError(
                    "Serialization of DBUserSecret can't fail",
                    Some(e.into()),
                )
            })?,
            SetCondition::None,
            SetExpiration::Ex(config::VERIFICATION_CODE_EXPIERY_SECONDS),
            false,
        )
        .forget();
    pipeline
        .set_with_options(
            tries_remaining_key,
            4,
            SetCondition::None,
            SetExpiration::Ex(config::VERIFICATION_CODE_EXPIERY_SECONDS),
            false,
        )
        .forget();
    pipeline
        .set_with_options(
            verification_code_key,
            code,
            SetCondition::None,
            SetExpiration::Ex(config::VERIFICATION_CODE_EXPIERY_SECONDS),
            false,
        )
        .forget();
    pipeline.execute().await?;

    // We are done
    Ok(())
}

// TODO: There are a bunch of seperate string allocation for keys
// maybe do those at once?
pub async fn verify_user(
    state: &State,
    username: String,
    code: u64,
) -> Result<DBUser, RtwalkError> {
    let user: Option<String> = state.redis.get(format!("pending:{}", &username)).await?;
    if user.is_none() {}
    if let Some(user) = user {
        let mut pipeline = state.redis.create_pipeline();
        pipeline
            .get::<_, ()>(format!("remaining_tries:{}", &username))
            .queue();
        pipeline
            .get::<_, ()>(format!("pending_secret:{}", &username))
            .queue();
        pipeline
            .get::<_, ()>(format!("verification_code:{}", &username))
            .queue();

        // Can this fail? If TTL expires between user fetch and this then yes,
        // In that case we say its an internal server error.
        let (remaining_tries, pending_secret, verification_code): (u64, String, u64) =
            pipeline.execute().await?;
        if remaining_tries == 0 {
            // Delete keys
            let mut pipeline = state.redis.create_pipeline();
            pipeline.del(format!("pending:{}", &username)).forget();
            pipeline
                .del(format!("remaining_tries:{}", &username))
                .forget();
            pipeline
                .del(format!("pending_secret:{}", &username))
                .forget();
            pipeline
                .del(format!("verification_code:{}", &username))
                .forget();
            pipeline.execute().await?;
            return Err(RtwalkError::VerificationCodeExpired);
        }

        // Compare verification code
        // TODO: Maybe change this to be black box comparison?
        if code != verification_code {
            // TODO: This might create a key without TTL if remaining_tries has already expired
            state
                .redis
                .decr(format!("remaining_tries:{}", &username))
                .await?;
            return Err(RtwalkError::InvalidVerificationCode);
        }
        // The user is REAL, make his account
        let user = create_user(
            state,
            serde_json::from_str(&user).map_err(|e| {
                RtwalkError::ImpossibleError(
                    "User serialized by server can't be invalid",
                    Some(e.into()),
                )
            })?,
            serde_json::from_str(&pending_secret).map_err(|e| {
                RtwalkError::ImpossibleError(
                    "UserSecret serialized by server can't be invalid",
                    Some(e.into()),
                )
            })?,
        )
        .await?;
        // Delete keys.
        let mut pipeline = state.redis.create_pipeline();
        pipeline.del(format!("pending:{}", &username)).forget();
        pipeline
            .del(format!("remaining_tries:{}", &username))
            .forget();
        pipeline
            .del(format!("pending_secret:{}", &username))
            .forget();
        pipeline
            .del(format!("verification_code:{}", &username))
            .forget();
        pipeline.execute().await?;
        // We are done
        return Ok(user);
    }
    Err(RtwalkError::VerificationCodeExpired)
}

async fn create_user(
    state: &State,
    user: DBUser,
    secret: DBUserSecret,
) -> Result<DBUser, RtwalkError> {
    state
        .db
        .query("BEGIN TRANSACTION")
        .query("CREATE user CONTENT $user")
        .query("CREATE user_secret CONTENT $secret")
        .query("COMMIT TRANSACTION")
        .bind(("user", &user))
        .bind(("secret", &secret))
        .await?;
    Ok(user)
}

pub async fn create_bot(
    state: &State,
    owner_id: String,
    username: String,
) -> Result<(String, DBUser), RtwalkError> {
    let mut exists = state
        .db
        .query("SELECT 1 FROM user WHERE username = $username")
        .bind(("username", &username))
        .await?;
    let username_exists: Option<u64> = exists.take((0, "1"))?;
    if username_exists.is_some() {
        return Err(RtwalkError::UsernameAlreadyExists);
    }
    // Make sure no one is trying to verify with the same suername
    let user: Option<String> = state.redis.get(format!("pending:{}", &username)).await?;
    if user.is_some() {
        return Err(RtwalkError::UsernameAlreadyExists);
    }

    let email = cuid();
    let password = cuid();

    let creds = format!("{}@{}", &email, &password);

    let password_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|p| p.to_string())
            .map_err(|e| RtwalkError::InternalError(e.into()))
    })
    .await
    .map_err(|e| RtwalkError::InternalError(e.into()))??;

    let bot = DBUser::new(
        username,
        true,
        Some(Thing {
            tb: "user".into(),
            id: owner_id.into(),
        }),
    );
    let secret = DBUserSecret {
        user: Thing {
            tb: "user".into(),
            id: bot.id.clone().into(),
        },
        email,
        password: password_hash,
    };

    let bot = create_user(state, bot, secret).await?;

    Ok((creds, bot))
}

// Returns the user if creds are correct, else return error
pub async fn login_user(
    state: &State,
    email: String,
    password: String,
) -> Result<DBUser, RtwalkError> {
    // First try to find user and their secret
    let mut res = state
        .db
        .query("SELECT password, user.* AS user FROM user_secret WHERE email = $email")
        .bind(("email", &email))
        .await?;
    let password_hash: Option<String> = res.take((0, "password"))?;
    if let Some(password_hash) = password_hash {
        if tokio::task::spawn_blocking(move || {
            PasswordHash::new(&password_hash)
                .map_err(|e| {
                    RtwalkError::ImpossibleError(
                        "Hash created by server can't be invalid",
                        Some(e.into()),
                    )
                })
                .map(|h| {
                    Argon2::default()
                        .verify_password(password.as_bytes(), &h)
                        .is_ok()
                })
        })
        .await
        .map_err(|e| RtwalkError::InternalError(e.into()))??
        {
            let user: Option<DBUser> = res.take((0, "user"))?;
            return Ok(user.ok_or(RtwalkError::ImpossibleError(
                "Secret exists but user doesnt",
                None,
            ))?);
        }
    }

    return Err(RtwalkError::InvalidCredentials);
}

pub async fn verify_bot_belongs_to_user(
    state: &State,
    user_id: &str,
    bot_id: &str,
) -> Result<DBUser, RtwalkError> {
    let mut bot = state
        .db
        .query("SELECT * FROM user WHERE id = $id")
        .bind((
            "id",
            Thing {
                tb: "user".into(),
                id: bot_id.into(),
            },
        ))
        .await?;
    let bot: Option<DBUser> = bot.take(0)?;

    if let Some(bot) = bot {
        if let Some(ref owner) = bot.owner {
            if owner.id.to_raw() == user_id {
                return Ok(bot);
            }
        }
    }
    Err(RtwalkError::UnauhorizedRequest)
}

pub async fn reset_bot_password(state: &State, bot_id: &str) -> Result<String, RtwalkError> {
    let password = cuid();

    let creds = format!("@{}", &password);

    let password_hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|p| p.to_string())
            .map_err(|e| RtwalkError::InternalError(e.into()))
    })
    .await
    .map_err(|e| RtwalkError::InternalError(e.into()))??;

    let mut res = state
        .db
        .query("UPDATE user_secret SET password = $password_hash WHERE user = $user RETURN email")
        .bind(("password_hash", password_hash))
        .bind((
            "user",
            Thing {
                tb: "user".into(),
                id: bot_id.into(),
            },
        ))
        .await?;

    let mut email =
        res.take::<Option<String>>((0, "email"))?
            .ok_or(RtwalkError::ImpossibleError(
                "Email exists if secret exists",
                None,
            ))?;
    email.push_str(&creds);

    Ok(email)
}

pub async fn reset_password(state: &State, email: &str) -> Result<(), RtwalkError> {
    let mut res = state
        .db
        .query("SELECT password, user.bot as bot FROM user_secret WHERE email = $email")
        .bind(("email", email))
        .await?;
    let password_hash: Option<String> = res.take((0, "password"))?;
    let bot: Option<bool> = res.take((0, "bot"))?;
    if let Some(password_hash) = password_hash {
        if bot.unwrap() {
            return Err(RtwalkError::UnauthenticatedRequest);
        }
        let token = PasetoBuilder::<V4, Local>::default()
            .set_claim(
                CustomClaim::try_from(("password_hash", password_hash)).map_err(|_| {
                    RtwalkError::ImpossibleError("Claim from (&str, &str) will be successful", None)
                })?,
            )
            .set_claim(CustomClaim::try_from(("email", email)).map_err(|_| {
                RtwalkError::ImpossibleError("Claim from (&str, &str) will be successful", None)
            })?)
            .build(&state.paseto_key)
            .map_err(|e| RtwalkError::InternalError(e.into()))?;
        // TODO: Send this token to the email
        dbg!(token);
    }
    Ok(())
}

pub async fn set_new_password(
    state: &State,
    token: &str,
    new_password: String,
) -> Result<(), RtwalkError> {
    let data = PasetoParser::<V4, Local>::default()
        .parse(&token, &state.paseto_key)
        .map_err(|_| RtwalkError::InvalidPasswordResetToken)?;

    let password_hash = data["password_hash"].to_string();
    let password_hash = &password_hash[1..password_hash.len() - 1];
    let email = data["email"].to_string();
    let email = &email[1..email.len() - 1];
    let mut res = state
        .db
        .query("SELECT 1 FROM user_secret WHERE password = $password_hash AND email = $email")
        .bind(("password_hash", password_hash))
        .bind(("email", email))
        .await?;
    let exists: Option<u64> = res.take((0, "1"))?;
    if exists.is_some() {
        let new_password_hash = tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            argon2
                .hash_password(new_password.as_bytes(), &salt)
                .map(|p| p.to_string())
                .map_err(|e| RtwalkError::InternalError(e.into()))
        })
        .await
        .map_err(|e| RtwalkError::InternalError(e.into()))??;

        state
            .db
            .query("UPDATE user_secret SET password = $new_password_hash WHERE email = $email")
            .bind(("new_password_hash", new_password_hash))
            .bind(("email", email))
            .await?;
        return Ok(());
    }
    Err(RtwalkError::InvalidPasswordResetToken)
}

pub async fn update_user(state: &State, updated_user: User) -> Result<DBUser, RtwalkError> {
    let mut db_user: DBUser = updated_user.into();
    db_user.modified_at = Datetime::default();

    let mut exists = state
        .db
        .query("SELECT 1 FROM user WHERE username = $username AND id != $id")
        .bind(("username", &db_user.username))
        .bind(("id", &db_user.id))
        .await?;
    let username_exists: Option<u64> = exists.take((0, "1"))?;
    if username_exists.is_some() {
        return Err(RtwalkError::UsernameAlreadyExists);
    }

    let res: Option<DBUser> = state.db.update(&db_user.id).content(db_user).await?;

    res.ok_or(RtwalkError::ImpossibleError("Failed at user update", None))
}

pub async fn fetch_user(
    state: &State,
    criteria: UserSelectCriteria,
) -> Result<Option<DBUser>, RtwalkError> {
    let user: Option<DBUser> = match criteria {
        UserSelectCriteria::Id(id) => state.db.select(("user", id)).await?,
        UserSelectCriteria::Username(username) => {
            let mut res = state
                .db
                .query("SELECT * FROM user WHERE username = $username")
                .bind(("username", username))
                .await?;

            res.take(0)?
        }
    };

    Ok(user)
}
