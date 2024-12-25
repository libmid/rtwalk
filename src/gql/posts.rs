use crate::{
    error::RtwalkError,
    gql::PageInfo,
    models::{file::File, post::DBPost, Key},
    state::State,
};
use surrealdb::RecordId;

pub async fn create_post(
    state: &State,
    title: String,
    tags: Vec<String>,
    content: Option<String>,
    attachments: Vec<File>,
    poster: Key,
    forum: Key,
) -> Result<DBPost, RtwalkError> {
    let forum = DBPost::new(title, tags, content, attachments, poster, forum);

    state
        .db
        .query("CREATE post CONTENT $post")
        .bind(("post", forum.clone()))
        .await?;

    Ok(forum)
}
