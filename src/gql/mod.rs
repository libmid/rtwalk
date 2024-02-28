use self::users::PasswordValidator;
use crate::{error::Result, models::user::User, State};
use async_graphql::{Context, Object, SimpleObject};

pub mod users;

macro_rules! state {
    ($ctx: ident) => {
        $ctx.data_unchecked::<State>()
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
        users::push_pending(state!(ctx), username, email, password).await?;
        Ok("Verification code sent to email")
    }

    async fn verify_user(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(min_length = 4, max_length = 20, regex = r"^[a-z0-9_]+$"))]
        username: String,
        code: u64,
    ) -> Result<User> {
        Ok(users::verify_user(state!(ctx), username, code)
            .await?
            .into())
    }
}
