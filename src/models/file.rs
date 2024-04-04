use async_graphql::{SimpleObject, UploadValue};
use opendal::Operator;
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(SimpleObject, Serialize, Deserialize, Debug)]
pub struct File {
    pub loc: String,
}

pub trait FileOps {
    async fn save(&self, op: &Operator, file: &mut UploadValue) -> anyhow::Result<()>;
    async fn delete(&self, op: &Operator) -> anyhow::Result<()>;
}

impl FileOps for File {
    async fn save(&self, op: &Operator, upload_value: &mut UploadValue) -> anyhow::Result<()> {
        let mut buffer = Vec::with_capacity(upload_value.size()? as usize);
        upload_value.content.read_to_end(&mut buffer)?;

        op.write(&self.loc, buffer).await?;

        Ok(())
    }

    async fn delete(&self, op: &Operator) -> anyhow::Result<()> {
        op.delete(&self.loc).await?;

        Ok(())
    }
}

impl FileOps for Option<File> {
    async fn save(&self, op: &Operator, file: &mut UploadValue) -> anyhow::Result<()> {
        if let Some(f) = self {
            f.save(op, file).await?;
        }
        Ok(())
    }

    async fn delete(&self, op: &Operator) -> anyhow::Result<()> {
        if let Some(f) = self {
            f.delete(op).await?;
        }
        Ok(())
    }
}
