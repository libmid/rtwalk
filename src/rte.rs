use std::env;

use async_stream::stream;
use axum::{
    response::sse::{Event, Sse},
    Json,
};
use bytes::Buf;
use futures::{Stream, StreamExt, TryStreamExt};
use rustis::{client::Client, commands::PubSubCommands};
use serde::Deserialize;

use crate::{error::RtwalkError, models::RtEvent};

#[derive(Deserialize)]
pub struct RtePayload {
    channels: Vec<String>,
}

pub async fn rte_sse_handler(
    Json(payload): Json<RtePayload>,
) -> Sse<impl Stream<Item = Result<Event, RtwalkError>>> {
    let mut sub_stream = Client::connect(env::var("REDIS_URL").expect("REDIS_URL"))
        .await
        .unwrap()
        .subscribe(payload.channels)
        .await
        .map_err(|e| RtwalkError::RedisError(e))
        .unwrap();
    // .extend_err(|_, _| {})?;

    let stream = stream! {
        while let Some(maybe_sub_msg) = sub_stream.next().await {
            if let Ok(sub_msg) = maybe_sub_msg {

                let event: RtEvent = serde_json::from_reader(sub_msg.payload.reader()).expect("Payload must be valid");

                yield Ok(event);
            }
            // TODO: Handle this error
        }
    };

    Sse::new(stream.map_ok(|e| Event::default().json_data(e).unwrap()))
}
