use std::borrow::Cow;

use crate::{
    error::RtwalkError,
    models::{forum::DBForum, Id},
    state::State,
};

pub async fn create_forum<'r>(
    state: &State,
    name: &Cow<'r, str>,
    owner: Id,
) -> Result<DBForum<'r>, RtwalkError> {
    let mut res = state
        .db
        .query("SELECT 1 FROM forum WHERE name = $name")
        .bind(("name", &name))
        .await?;

    let exists: Option<u64> = res.take((0, "1"))?;

    if exists.is_some() {
        return Err(RtwalkError::ForumAlreadyExists);
    }

    let forum = DBForum::new(name, owner);

    state
        .db
        .query("CREATE forum CONTENT $forum")
        .bind(("forum", &forum))
        .await?;

    Ok(forum)
}
