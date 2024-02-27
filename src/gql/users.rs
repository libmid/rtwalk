use crate::config;
use crate::template::EmailVerify;
use crate::{
    error::RtwalkError,
    models::user::{db, secret_db, DBUser, DBUserSecret},
    State,
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_graphql::CustomValidator;
use mongodm::{doc, f};
use rand::Rng;

use rustis::{
    client::BatchPreparedCommand,
    commands::{SetCondition, SetExpiration, StringCommands},
};

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
        format!("tries:{}", &username),
        format!("pending_secret:{}", &username),
        format!("verification_code:{}", &username),
    );
    let user = DBUser::new(username, false);
    let secret = DBUserSecret {
        user_id: None,
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

pub async fn verify_user(
    state: &State,
    username: String,
    code: u64,
) -> Result<DBUser, RtwalkError> {
    let user: Option<String> = state.redis.get(format!("pending:{}", &username)).await?;
    if user.is_none() {
        return Err(RtwalkError::VerificationCodeExpired);
    }
    let mut pipeline = state.redis.create_pipeline();
    pipeline
        .get::<_, ()>(format!("tries:{}", &username))
        .queue();
    pipeline
        .get::<_, ()>(format!("pending_secret:{}", &username))
        .queue();
    pipeline
        .get::<_, ()>(format!("verification_code:{}", &username))
        .queue();

    let (tries, pending_secret, verification_code): (u64, String, u64) = pipeline.execute().await?;
    todo!()
}
