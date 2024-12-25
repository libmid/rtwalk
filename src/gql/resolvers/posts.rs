use async_graphql::{Context, Object, OneofObject, ResultExt, Upload};
use cuid2::cuid;

use crate::{
    config,
    error::RtwalkError,
    gql::{posts, state, user},
    models::{
        file::{File, FileOps},
        forum::{DBForum, Forum},
        post::{DBPost, Post},
        user::DBUser,
        Key,
    },
};

use super::super::Role;

#[derive(Default)]
pub struct PostMutationRoot;

#[Object]
impl PostMutationRoot {
    #[graphql(guard = Role::Authenticated)]
    async fn create_post(
        &self,
        ctx: &Context<'_>,
        forum: Key,
        #[graphql(validator(min_length = 1, max_length = 128))] title: String,
        #[graphql(validator(max_items = 6, list, max_length = 20))] tags: Vec<String>,
        #[graphql(validator(max_length = 10_000))] content: Option<String>,
        attachments: Vec<Upload>,
    ) -> async_graphql::Result<Post> {
        let user = user!(ctx);
        let state = state!(ctx);

        let mut uploads = vec![];
        for v in attachments {
            let mut upload_value = v.value(&ctx)?;
            if upload_value.size()? > config::MAX_UPLOAD_SIZE {
                return Err(RtwalkError::MaxUploadSizeExceeded).extend_err(|_, _| {})?;
            }

            let f = File {
                loc: format!(
                    "{}/{}-{}",
                    user.id.to_string(),
                    cuid(),
                    upload_value.filename
                ),
            };
            f.save(&state.op, &mut upload_value)
                .await
                .extend_err(|_, _| {})?;

            uploads.push(f);
        }

        let post = posts::create_post(&state, title, tags, content, uploads, user.id, forum)
            .await
            .extend_err(|_, _| {})?;

        Ok(post.into())
    }
}
