use crate::{
    error::RtwalkError,
    gql::PageInfo,
    models::{comment::DBComment, file::File, Key},
    state::State,
};
use surrealdb::types::RecordId;

use super::resolvers::comments::MultipleCommentSelectCriteria;

pub async fn create_comment(
    state: &State,
    content: Option<String>,
    attachments: Vec<File>,
    commenter: Key,
    post: Key,
) -> Result<DBComment, RtwalkError> {
    let comment = DBComment::new(content, attachments, commenter, post);

    state
        .db
        .query("CREATE comment CONTENT $comment")
        .bind(("comment", comment.clone()))
        .await?;

    Ok(comment)
}

pub async fn fetch_comments(
    state: &State,
    criteria: MultipleCommentSelectCriteria,
    page_info: &PageInfo,
) -> Result<Vec<DBComment>, RtwalkError> {
    let mut queries = vec![];
    let comments: Vec<DBComment> = match criteria {
        MultipleCommentSelectCriteria::Post(id) => {
            queries.clear();
            queries
                .push("SELECT * FROM comment WHERE post.id = $post_id LIMIT $limit START $start");

            if page_info.needs_page_info {
                queries.push("SELECT count() as total FROM comment WHERE post.id = $forum_id");
            }

            let query = state.db.query(queries.join(";"));
            let mut res = query
                .bind(("post_id", RecordId::new("post", id.0)))
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
        MultipleCommentSelectCriteria::Search(search) => {
            queries.clear();
            queries.push("SELECT * FROM comment WHERE content @1@ $query ORDER BY created_at ASC LIMIT $limit START $start");
            if page_info.needs_page_info {
                queries.push("SELECT count() as total FROM comment WHERE content @1@ $query")
            }
            let query = state.db.query(queries.join(";"));
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

    Ok(comments)
}
