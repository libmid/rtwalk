use async_graphql::{Context, Object, ResultExt};

use crate::{
    gql::{forums, state, user},
    models::forum::Forum,
};

use super::super::Role;

#[derive(Default)]
pub struct ForumMutationRoot;

#[Object]
impl ForumMutationRoot {
    #[graphql(guard = Role::Human)]
    async fn create_forum(&self, ctx: &Context<'_>, name: String) -> async_graphql::Result<Forum> {
        let user = user!(ctx);
        let state = state!(ctx);

        let user = forums::create_forum(&state, &name.into(), user.id)
            .await
            .extend_err(|_, _| {})?;

        Ok(user.into())
    }
}
