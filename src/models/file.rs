use async_graphql::SimpleObject;
use bytes::Bytes;
use opendal::{Operator, Result};
use serde::{Deserialize, Serialize};

#[derive(SimpleObject, Serialize, Deserialize)]
pub struct File {
    loc: String,
}

trait FileOps {
    async fn save(self, op: Operator, bs: impl Into<Bytes>) -> Result<()>;
}

impl FileOps for File {
    async fn save(self, op: Operator, bs: impl Into<Bytes>) -> Result<()> {
        op.write(&self.loc, bs).await?;

        Ok(())
    }
}
