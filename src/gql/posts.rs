use crate::{
    error::RtwalkError,
    gql::PageInfo,
    models::{file::File, post::DBPost, Key},
    state::State,
};
use surrealdb::RecordId;

use super::resolvers::posts::{MultiplePostSelectCriteria, PostSelectCriteria};

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

pub async fn fetch_post(
    state: &State,
    criteria: PostSelectCriteria,
) -> Result<Option<DBPost>, RtwalkError> {
    let post: Option<DBPost> = match criteria {
        PostSelectCriteria::Id(id) => state.db.select(("post", id.0)).await?,
    };

    Ok(post)
}

pub async fn fetch_posts(
    state: &State,
    criteria: MultiplePostSelectCriteria,
    page_info: &PageInfo,
) -> Result<Vec<DBPost>, RtwalkError> {
    let posts: Vec<DBPost> = match criteria {
        MultiplePostSelectCriteria::Ids(ids) => {
            let mut query = state
                .db
                .query("SELECT * FROM $ids LIMIT $limit START $start");

            if page_info.needs_page_info {
                query = query.query("SELECT count() as total FROM $ids");
            }

            let mut res = query
                .bind((
                    "ids",
                    ids.into_iter()
                        .map(|x| RecordId::from_table_key("post", x.0))
                        .collect::<Vec<_>>(),
                ))
                .bind(("limit", page_info.per_page))
                .bind(("start", (page_info.page - 1) * page_info.per_page))
                .await?;

            if page_info.needs_page_info {
                if let Some(total) = res.take((1, "total"))? {
                    page_info
                        .total
                        .0
                        .store(total, std::sync::atomic::Ordering::Relaxed);
                    page_info.has_next_page.0.store(
                        total > (page_info.page - 1) * page_info.per_page + page_info.per_page,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }
            }

            res.take(0)?
        }
        MultiplePostSelectCriteria::Forum(id) => {
            let mut query = state
                .db
                .query("SELECT * FROM post WHERE forum.id = $forum_id LIMIT $limit START $start");

            if page_info.needs_page_info {
                query = query.query("SELECT count() as total FROM post WHERE forum.id = $forum_id");
            }

            let mut res = query
                .bind(("forum_id", RecordId::from_table_key("forum", id.0)))
                .bind(("limit", page_info.per_page))
                .bind(("start", (page_info.page - 1) * page_info.per_page))
                .await?;

            if page_info.needs_page_info {
                if let Some(total) = res.take((1, "total"))? {
                    page_info
                        .total
                        .0
                        .store(total, std::sync::atomic::Ordering::Relaxed);
                    page_info.has_next_page.0.store(
                        total > (page_info.page - 1) * page_info.per_page + page_info.per_page,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }
            }

            res.take(0)?
        }
        MultiplePostSelectCriteria::Search(search) => {
            let mut query = state
                .db
                .query("SELECT * FROM post WHERE title @0@ $query OR content @1@ $query ORDER BY created_at ASC LIMIT $limit START $start");
            if page_info.needs_page_info {
                query = query.query("SELECT count() as total FROM post WHERE title @0@ $query OR content @1@ $query")
            }
            let mut res = query
                .bind(("query", search))
                .bind(("limit", page_info.per_page))
                .bind(("start", (page_info.page - 1) * page_info.per_page))
                .await?;

            if page_info.needs_page_info {
                if let Some(total) = res.take((1, "total"))? {
                    page_info
                        .total
                        .0
                        .store(total, std::sync::atomic::Ordering::Relaxed);
                    page_info.has_next_page.0.store(
                        total > (page_info.page - 1) * page_info.per_page + page_info.per_page,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }
            }

            res.take(0)?
        }
    };

    Ok(posts)
}
