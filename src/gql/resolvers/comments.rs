use async_graphql::{Context, MaybeUndefined, Object, OneofObject, ResultExt, Upload};
use chrono::DateTime;
use cuid2::cuid;
use rustis::commands::PubSubCommands;

use crate::{
    config,
    error::RtwalkError,
    gql::{comments, state, user},
    models::{
        comment::{Comment, DBComment},
        file::{File, FileOps},
        CommentCreateEvent, CommentEditEvent, Key, RtEvent, RtEventData, RtEventType,
    },
};

use super::super::Role;

#[derive(Default)]
pub struct CommentMutationRoot;

#[Object]
impl CommentMutationRoot {
    #[graphql(guard = Role::Authenticated)]
    async fn create_comment(
        &self,
        ctx: &Context<'_>,
        post: Key,
        #[graphql(validator(min_length = 1, max_length = 1_000))] content: Option<String>,
        attachments: Vec<Upload>,
    ) -> async_graphql::Result<Comment> {
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

        let comment: Comment = comments::create_comment(&state, content, uploads, user.id, post)
            .await
            .extend_err(|_, _| {})?
            .into();

        state.redis.publish(
            "rte-comment-create",
            serde_json::to_vec(&RtEvent {
                ty: RtEventType::CommentCreate,
                event_data: RtEventData::CommentCreate(CommentCreateEvent {
                    data: comment.clone(),
                }),
            })
            .expect("Cant fail to serialize self constructed data"),
        );

        Ok(comment)
    }

    #[graphql(guard = Role::Authenticated)]
    async fn update_comment<'r>(
        &self,
        ctx: &Context<'r>,
        comment_id: Key,
        #[graphql(validator(min_length = 1, max_length = 1_000))] content: MaybeUndefined<String>,
        remove_attachments: bool,
    ) -> async_graphql::Result<Comment> {
        let state = state!(ctx);
        let user = user!(ctx);

        let comment: Option<DBComment> = state.db.select(("comment", comment_id.0)).await?;

        if let Some(mut comment) = comment {
            let original_comment: Comment = comment.clone().into();

            if &user.id.0 != comment.commenter.key() {
                return Err(RtwalkError::UnauhorizedRequest).extend_err(|_, _| {});
            }

            if content.is_null() {
                comment.content = None;
            } else if let MaybeUndefined::Value(content) = content {
                comment.content = Some(content);
            }

            if remove_attachments {
                for attachment in &comment.attachments {
                    attachment.delete(&state.op).await.extend_err(|_, _| {})?;
                }
            }

            comment.edited_at = DateTime::default();

            let res: Option<DBComment> = state.db.update(&comment.id).content(comment).await?;

            let updated_comment: Comment = res.expect("Comment exists").into();

            state.redis.publish(
                "rte-post-update",
                serde_json::to_vec(&RtEvent {
                    ty: RtEventType::CommentEdit,
                    event_data: RtEventData::CommentEdit(CommentEditEvent {
                        original: original_comment,
                        new: updated_comment.clone(),
                    }),
                })
                .expect("Cant fail to serialize self constructed data"),
            );

            Ok(updated_comment)
        } else {
            Err(RtwalkError::CommentNotFound).extend_err(|_, _| {})
        }
    }
}

// TODO: Add more useful criterias
#[derive(OneofObject)]
pub enum MultipleCommentSelectCriteria {
    Post(Key),
    Search(String),
}
