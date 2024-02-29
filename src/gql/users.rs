use crate::config;
use crate::template::EmailVerify;
use crate::{
    error::RtwalkError,
    models::user::{db, secret_db, DBUser, DBUserSecret},
    state::State,
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_graphql::CustomValidator;
use mongodm::prelude::*;
use rand::Rng;

use rustis::commands::GenericCommands;
use rustis::{
    client::BatchPreparedCommand,
    commands::{SetCondition, SetExpiration, StringCommands},
};

use mongodm::mongo::bson;
use sailfish::TemplateOnce;
use zxcvbn::zxcvbn;

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
    let user = db!(state.mongo)
        .find_one(doc! {f! (username in DBUser): &username}, None)
        .await?;
    if user.is_some() {
        return Err(RtwalkError::UsernameAlreadyExists);
    }
    // Make sure no one is trying to verify with the same suername
    let user: Option<String> = state.redis.get(format!("pending:{}", &username)).await?;
    if user.is_some() {
        return Err(RtwalkError::UsernameAlreadyExists);
    }

    // Check if user with same email already exists.
    let user = secret_db!(state.mongo)
        .find_one(doc! {f!(email in DBUserSecret): &email}, None)
        .await?;
    if user.is_some() {
        // Silently drop.
        return Ok(());
    }
    // Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("cant fail")
        .to_string();
    // We have verified and confirmed user is valid, generate verification code.
    let code = rand::thread_rng().gen_range(10000..=99999);
    // Send email to the user
    let _template = EmailVerify {
        username: &username,
        code,
        site_name: state.site_name,
    }
    .render_once()
    .expect("Can't fail");
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
    let user = DBUser::new(username, false);
    let secret = DBUserSecret {
        user_id: user.id.clone(),
        email,
        password: password_hash,
    };
    // Push the state into redis with a ttl.
    let mut pipeline = state.redis.create_pipeline();
    pipeline
        .set_with_options(
            pending_user_key,
            serde_json::to_string(&user).expect("Unexpected serialization error of DBUser"),
            SetCondition::None,
            SetExpiration::Ex(config::VERIFICATION_CODE_EXPIERY_SECONDS),
            false,
        )
        .forget();
    pipeline
        .set_with_options(
            secret_key,
            serde_json::to_string(&secret).expect("Can't fail"),
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
    if user.is_none() {
        return Err(RtwalkError::VerificationCodeExpired);
    }
    let user = user.expect("Can't fail");
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
        serde_json::from_str(&user).expect("Serialized by server. Can't fail."),
        serde_json::from_str(&pending_secret).expect("Serialized by server. Can't fail."),
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
    Ok(user)
}

#[inline]
async fn create_user(
    state: &State,
    user: DBUser,
    secret: DBUserSecret,
) -> Result<DBUser, RtwalkError> {
    db!(state.mongo).insert_one(&user, None).await?;
    secret_db!(state.mongo).insert_one(secret, None).await?;
    Ok(user)
}

// Returns the user if creds are correct, else return error
pub async fn login_user(
    state: &State,
    email: String,
    password: String,
) -> Result<DBUser, RtwalkError> {
    // First try to find user and their secret
    let mut user_secret_stream = secret_db!(state.mongo)
        .aggregate(
            pipeline![
                Match: { f!(email in DBUserSecret): &email},
                Lookup {
                From: "DBUser",
                As: "user",
                LocalField: "user_id",
                ForeignField: "id",
                }
            ],
            None,
        )
        .await?;
    if let Some(doc) = user_secret_stream.next().await {
        let mut doc = doc?;
        let parsed_hash = PasswordHash::new(
            doc.get("password")
                .expect("Can't fail")
                .as_str()
                .expect("can't fail"),
        )
        .expect("Can't fail");
        if Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            return Ok(bson::from_bson(
                doc.get_mut("user")
                    .expect("Can't fail")
                    .as_array_mut()
                    .expect("Can't fail")
                    .pop()
                    .expect("Can't fail"),
            )
            .expect("Can't fail"));
        }
    } else {
        return Err(RtwalkError::InvalidCredentials);
    }
    return Err(RtwalkError::InvalidCredentials);
}
