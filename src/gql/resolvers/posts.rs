use async_graphql::{Context, MaybeUndefined, Object, OneofObject, ResultExt, Upload};
use chrono::DateTime;
use cuid2::cuid;
use rustis::commands::PubSubCommands;

use crate::{
    config,
    error::RtwalkError,
    gql::{posts, state, user},
    models::{
        file::{File, FileOps},
        post::{DBPost, Post},
        Key, PostCreateEvent, PostEditEvent, RtEvent, RtEventData, RtEventType,
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
        #[graphql(validator(max_items = 8, list, max_length = 20))] tags: Vec<String>,
        #[graphql(validator(min_length = 1, max_length = 8_000))] content: Option<String>,
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

        let post: Post = posts::create_post(&state, title, tags, content, uploads, user.id, forum)
            .await
            .extend_err(|_, _| {})?
            .into();

        state.redis.publish(
            "rte-post-create",
            serde_json::to_vec(&RtEvent {
                ty: RtEventType::PostCreate,
                event_data: RtEventData::PostCreate(PostCreateEvent { data: post.clone() }),
            })
            .expect("Cant fail to serialize self constructed data"),
        );

        Ok(post)
    }

    #[graphql(guard = Role::Authenticated)]
    async fn update_post<'r>(
        &self,
        ctx: &Context<'r>,
        post_id: Key,
        #[graphql(validator(min_length = 1, max_length = 128))] title: Option<String>,
        #[graphql(validator(max_items = 8, list, max_length = 20))] tags: Option<Vec<String>>,
        #[graphql(validator(min_length = 1, max_length = 8_000))] content: MaybeUndefined<String>,
        remove_attachments: bool,
    ) -> async_graphql::Result<Post> {
        let state = state!(ctx);
        let user = user!(ctx);

        let post: Option<DBPost> = state.db.select(("post", post_id.0)).await?;

        if let Some(mut post) = post {
            let original_post: Post = post.clone().into();

            if &user.id.0 != post.poster.key() {
                return Err(RtwalkError::UnauhorizedRequest).extend_err(|_, _| {});
            }

            if let Some(title) = title {
                post.title = title;
            }

            if let Some(tags) = tags {
                post.tags = tags;
            }

            if content.is_null() {
                post.content = None;
            } else if let MaybeUndefined::Value(content) = content {
                post.content = Some(content);
            }

            if remove_attachments {
                for attachment in &post.attachments {
                    attachment.delete(&state.op).await.extend_err(|_, _| {})?;
                }
            }

            post.edited_at = DateTime::default();

            let res: Option<DBPost> = state.db.update(&post.id).content(post).await?;

            let updated_post: Post = res.expect("Post exists").into();

            state.redis.publish(
                "rte-post-update",
                serde_json::to_vec(&RtEvent {
                    ty: RtEventType::PostEdit,
                    event_data: RtEventData::PostEdit(PostEditEvent {
                        original: original_post,
                        new: updated_post.clone(),
                    }),
                })
                .expect("Cant fail to serialize self constructed data"),
            );

            Ok(updated_post)
        } else {
            Err(RtwalkError::PostNotFound).extend_err(|_, _| {})
        }
    }
}

#[derive(Default)]
pub struct PostQueryRoot;

#[derive(OneofObject)]
pub enum PostSelectCriteria {
    Id(Key),
}

// TODO: Add more useful criterias
#[derive(OneofObject)]
pub enum MultiplePostSelectCriteria {
    Ids(Vec<Key>),
    Forum(Key),
    Search(String),
}

#[Object]
impl PostQueryRoot {
    async fn post<'r>(
        &self,
        ctx: &Context<'r>,
        criteria: PostSelectCriteria,
    ) -> async_graphql::Result<Option<Post>> {
        let state = state!(ctx);

        let post = posts::fetch_post(state, criteria)
            .await
            .extend_err(|_, _| {})?;

        Ok(post.map(|x| x.into()))
    }
}
