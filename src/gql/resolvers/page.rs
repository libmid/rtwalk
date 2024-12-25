use crate::{
    error::RtwalkError,
    gql::{
        forums,
        resolvers::{forums::MultipleForumSelectCriteria, users::MultipleUserSelectCriteria},
        state, user, users, Page, Role,
    },
    models::{file::File, forum::Forum, user::User},
};
use async_graphql::{ComplexObject, Context, ResultExt};

#[ComplexObject]
impl Page {
    async fn user(
        &self,
        ctx: &Context<'_>,
        criteria: MultipleUserSelectCriteria,
    ) -> async_graphql::Result<Vec<User>> {
        let state = state!(ctx);
        let users = users::fetch_users(state, criteria, &self.page_info)
            .await
            .extend_err(|_, _| {})?;
        Ok(users.into_iter().map(|x| x.into()).collect())
    }

    #[graphql(guard = Role::Authenticated)]
    async fn file(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<File>> {
        let state = state!(ctx);
        let user = user!(ctx);
        let files = state
            .op
            .list_with(&(user.id.to_string() + "/"))
            .await
            .map_err(|e| RtwalkError::OpendalError(e))
            .extend_err(|_, _| {})?
            .into_iter()
            .skip(((self.page_info.page - 1) * self.page_info.per_page) as usize)
            .take(self.page_info.per_page as usize)
            .map(|f| File {
                loc: f.path().to_string() + f.name(),
            })
            .collect();

        Ok(files)
    }

    async fn forum(
        &self,
        ctx: &Context<'_>,
        criteria: MultipleForumSelectCriteria,
    ) -> async_graphql::Result<Vec<Forum>> {
        let state = state!(ctx);
        let forums = forums::fetch_forums(state, criteria, &self.page_info)
            .await
            .extend_err(|_, _| {})?;
        Ok(forums.into_iter().map(|x| x.into()).collect())
    }
}
