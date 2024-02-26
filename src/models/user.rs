use mongodm::{bson::oid::ObjectId, field, CollectionConfig, Index, Model};

pub struct DBUser {
    id: ObjectId,
    username: String,
    display_name: String,
}
