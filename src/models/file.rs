use async_graphql::{SimpleObject, UploadValue};
use opendal::Operator;
use serde::{Deserialize, Serialize};
use std::io::Read;

use crate::error::RtwalkError;

#[derive(SimpleObject, Serialize, Deserialize, Debug, Clone)]
pub struct File {
    pub loc: String,
}

pub trait FileOps {
    async fn save(&self, op: &Operator, file: &mut UploadValue) -> Result<(), RtwalkError>;
    async fn delete(&self, op: &Operator) -> Result<(), RtwalkError>;
}

impl FileOps for File {
    async fn save(&self, op: &Operator, upload_value: &mut UploadValue) -> Result<(), RtwalkError> {
        let mut buffer = Vec::with_capacity(
            upload_value
                .size()
                .map_err(|e| RtwalkError::InternalError(e.into()))? as usize,
        );
        upload_value
            .content
            .read_to_end(&mut buffer)
            .map_err(|e| RtwalkError::InternalError(e.into()))?;

        op.write(&self.loc, buffer).await?;

        Ok(())
    }

    async fn delete(&self, op: &Operator) -> Result<(), RtwalkError> {
        op.delete(&self.loc).await?;

        Ok(())
    }
}

impl FileOps for Option<File> {
    async fn save(&self, op: &Operator, file: &mut UploadValue) -> Result<(), RtwalkError> {
        if let Some(f) = self {
            f.save(op, file).await?;
        }
        Ok(())
    }

    async fn delete(&self, op: &Operator) -> Result<(), RtwalkError> {
        if let Some(f) = self {
            f.delete(op).await?;
        }
        Ok(())
    }
}
