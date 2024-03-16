use anyhow::Result;
use async_graphql::EmptySubscription;
use async_graphql::Schema;
use dotenvy::dotenv;
use rustis::client::Client;
use std::env;
use std::sync::Arc;
use surrealdb::engine::local::Mem;
use surrealdb::opt::auth::Database;
use surrealdb::Surreal;
use tower_cookies::Key;

use rtwalk::ApiInfo;
use rtwalk::InnerState;
use rtwalk::MutationRoot;
use rtwalk::QueryRoot;
use rtwalk::State;

pub async fn setup() -> Result<Schema<QueryRoot, MutationRoot, EmptySubscription>> {
    dotenv()?;

    let redis = Client::connect(env::var("REDIS_URL").expect("REDIS_URL")).await?;
    let pubsub_redis = Client::connect(env::var("REDIS_URL").expect("REDIS_URL")).await?;
    let surreal_client = Surreal::new::<Mem>(()).await?;
    surreal_client.use_ns("test").use_db("rtwalk").await?;

    let cookies_key = env::var("COOKIE_KEY").expect("COOKIE_KEY");

    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(State {
            inner: Arc::new(InnerState {
                site_name: "DreamH",
                info: ApiInfo {
                    major: 0,
                    minor: 1,
                    bugfix: 0,
                    rte: "ws://localhost:4001/rte",
                    vc: "ws://localhost:4002/ws",
                },
                redis,
                pubsub: pubsub_redis,
                // Its Arc internally so its fine to clone
                db: surreal_client.into(),
                cookie_key: Key::from(cookies_key.as_bytes()),
            }),
        })
        .finish();

    Ok(schema)
}
