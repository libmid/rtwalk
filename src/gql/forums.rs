use crate::{
    error::RtwalkError,
    gql::PageInfo,
    models::{forum::DBForum, Key},
    state::State,
};
use surrealdb::RecordId;

use super::resolvers::forums::{ForumSelectCriteria, MultipleForumSelectCriteria};

pub async fn create_forum(state: &State, name: String, owner: Key) -> Result<DBForum, RtwalkError> {
    let mut res = state
        .db
        .query("SELECT 1 FROM forum WHERE name = $name")
        .bind(("name", name.clone()))
        .await?;

    let exists: Option<u64> = res.take((0, "1"))?;

    if exists.is_some() {
        return Err(RtwalkError::ForumAlreadyExists);
    }

    let forum = DBForum::new(&name, owner);

    state
        .db
        .query("CREATE forum CONTENT $forum")
        .bind(("forum", forum.clone()))
        .await?;

    Ok(forum)
}

pub async fn fetch_forum(
    state: &State,
    criteria: ForumSelectCriteria,
) -> Result<Option<DBForum>, RtwalkError> {
    let forum: Option<DBForum> = match criteria {
        ForumSelectCriteria::Id(id) => state.db.select(("forum", id)).await?,
        ForumSelectCriteria::Name(name) => {
            let mut res = state
                .db
                .query("SELECT * FROM forum WHERE name = $name")
                .bind(("name", name))
                .await?;

            res.take(0)?
        }
    };

    Ok(forum)
}

pub async fn fetch_forums(
    state: &State,
    criteria: MultipleForumSelectCriteria,
    page_info: &PageInfo,
) -> Result<Vec<DBForum>, RtwalkError> {
    let forums: Vec<DBForum> = match criteria {
        MultipleForumSelectCriteria::Ids(ids) => {
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
                        .map(|x| RecordId::from_table_key("forum", x))
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
        MultipleForumSelectCriteria::Names(names) => {
            let mut query = state
                .db
                .query("SELECT * FROM forum WHERE name IN $names LIMIT $limit START $start");

            if page_info.needs_page_info {
                query = query.query("SELECT count() as total FROM forum WHERE name IN $names");
            }

            let mut res = query
                .bind(("names", names))
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
        MultipleForumSelectCriteria::Search(search) => match search.as_str() {
            "*" => {
                let mut query = state
                    .db
                    .query("SELECT * FROM forum ORDER BY created_at ASC LIMIT $limit START $start");
                if page_info.needs_page_info {
                    query = query.query("SELECT count() as total FROM forum")
                }
                let mut res = query
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
            _ => {
                let mut query = state
                .db
                .query("SELECT * FROM forum WHERE name @0@ $query OR display_name @1@ $query OR description @2@ $query ORDER BY created_at ASC LIMIT $limit START $start");
                if page_info.needs_page_info {
                    query = query.query("SELECT count() as total FROM forum WHERE name @0@ $query OR display_name @1@ $query OR description @2@ $query")
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
        },
    };

    Ok(forums)
}
