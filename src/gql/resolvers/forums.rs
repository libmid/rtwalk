use async_graphql::{Context, MaybeUndefined, Object, OneofObject, ResultExt, Upload};
use cuid2::cuid;

use crate::{
    config,
    error::RtwalkError,
    gql::{forums, state, user},
    models::{
        file::{File, FileOps},
        forum::{DBForum, Forum},
        user::DBUser,
        Key,
    },
};

use super::super::Role;

#[derive(Default)]
pub struct ForumMutationRoot;

#[Object]
impl ForumMutationRoot {
    #[graphql(guard = Role::Human)]
    async fn create_forum(
        &self,
        ctx: &Context<'_>,
        #[graphql(validator(min_length = 3, max_length = 20, regex = r"^[a-z0-9_]+$"))]
        name: String,
    ) -> async_graphql::Result<Forum> {
        let user = user!(ctx);
        let state = state!(ctx);

        let forum = forums::create_forum(&state, name.into(), user.id)
            .await
            .extend_err(|_, _| {})?;

        Ok(forum.into())
    }

    #[graphql(guard = Role::Human)]
    async fn update_forum<'r>(
        &self,
        ctx: &Context<'r>,
        forum_id: Key,
        #[graphql(validator(min_length = 3, max_length = 20, regex = r"^[a-z0-9_]+$"))]
        name: Option<String>,
        display_name: Option<String>,
        description: MaybeUndefined<String>,
        icon: MaybeUndefined<Upload>,
        banner: MaybeUndefined<Upload>,
    ) -> async_graphql::Result<Forum> {
        let state = state!(ctx);
        let user = user!(ctx);

        if user.id != forum_id {
            return Err(RtwalkError::UnauhorizedRequest).extend_err(|_, _| {});
        }

        let forum: Option<DBForum> = state.db.select(("forum", forum_id.0)).await?;

        if let Some(mut forum) = forum {
            if let Some(name) = name {
                forum.name = name;
            }

            if let Some(display_name) = display_name {
                forum.display_name = display_name;
            }

            if description.is_null() {
                forum.description = None;
            } else if let MaybeUndefined::Value(description) = description {
                forum.description = Some(description);
            }

            if icon.is_null() {
                forum.icon.delete(&state.op).await.extend_err(|_, _| {})?;

                forum.icon = None;
            } else if let MaybeUndefined::Value(v) = icon {
                let mut upload_value = v.value(&ctx)?;
                if upload_value.size()? > config::MAX_UPLOAD_SIZE {
                    return Err(RtwalkError::MaxUploadSizeExceeded).extend_err(|_, _| {})?;
                }

                forum.icon.delete(&state.op).await.extend_err(|_, _| {})?;

                let icon_file = File {
                    loc: format!("{}/{}-{}", forum.id, cuid(), upload_value.filename),
                };
                icon_file
                    .save(&state.op, &mut upload_value)
                    .await
                    .extend_err(|_, _| {})?;
                forum.icon = Some(icon_file);
            }

            if banner.is_null() {
                forum.banner.delete(&state.op).await.extend_err(|_, _| {})?;

                forum.banner = None;
            } else if let MaybeUndefined::Value(v) = banner {
                let mut upload_value = v.value(&ctx)?;
                if upload_value.size()? > config::MAX_UPLOAD_SIZE {
                    return Err(RtwalkError::MaxUploadSizeExceeded).extend_err(|_, _| {})?;
                }

                forum.banner.delete(&state.op).await.extend_err(|_, _| {})?;

                let banner_file = File {
                    loc: format!("{}/{}-{}", &forum.id, cuid(), upload_value.filename),
                };
                banner_file
                    .save(&state.op, &mut upload_value)
                    .await
                    .extend_err(|_, _| {})?;
                forum.banner = Some(banner_file);
            }

            let res: Option<DBForum> = state.db.update(&forum.id).content(forum).await?;
            Ok(res.expect("Forum exists").into())
        } else {
            Err(RtwalkError::ForumNotFound).extend_err(|_, _| {})
        }
    }

    #[graphql(guard = Role::Human)]
    async fn add_moderator<'r>(
        &self,
        ctx: &Context<'r>,
        forum_id: Key,
        mod_id: Key,
    ) -> async_graphql::Result<String> {
        let state = state!(ctx);
        let user = user!(ctx);

        let forum: Option<DBForum> = state.db.select(("forum", forum_id.0)).await?;
        let new_mod: Option<DBUser> = state.db.select(("user", mod_id.0)).await?;

        todo!()
    }
}

#[derive(Default)]
pub struct ForumQueryRoot;

#[derive(OneofObject)]
pub enum ForumSelectCriteria {
    Id(String),
    Name(String),
}

#[derive(OneofObject)]
pub enum MultipleForumSelectCriteria {
    Ids(Vec<String>),
    Names(Vec<String>),
    Search(String),
}

#[Object]
impl ForumQueryRoot {
    async fn forum<'r>(
        &self,
        ctx: &Context<'r>,
        criteria: ForumSelectCriteria,
    ) -> async_graphql::Result<Option<Forum>> {
        let state = state!(ctx);

        let forum = forums::fetch_forum(state, criteria)
            .await
            .extend_err(|_, _| {})?;

        Ok(forum.map(|x| x.into()))
    }
}
