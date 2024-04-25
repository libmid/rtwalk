use async_graphql::{Context, MaybeUndefined, Object, ResultExt, Upload};

use crate::{
    gql::{forums, state, user},
    models::{forum::Forum, Id},
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

    #[graphql(guard = Role::Human)]
    async fn update_forum<'r>(
        &self,
        ctx: &Context<'r>,
        forum_id: Id,
        #[graphql(validator(min_length = 4, max_length = 20, regex = r"^[a-z0-9_]+$"))]
        name: Option<String>,
        display_name: Option<String>,
        description: MaybeUndefined<String>,
        icon: MaybeUndefined<Upload>,
        banner: MaybeUndefined<Upload>,
    ) -> async_graphql::Result<Forum<'r>> {
        todo!()
    }
}
