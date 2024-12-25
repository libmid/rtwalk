use anyhow::Result;
use async_graphql::EmptySubscription;
use async_graphql::Schema;
use dotenvy::dotenv;
use opendal::Operator;
use rustis::client::Client;
use rusty_paseto::generic::Local;
use rusty_paseto::generic::PasetoSymmetricKey;
use rusty_paseto::generic::V4;
use std::env;
use std::sync::Arc;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Database;
use surrealdb::Surreal;
use tower_cookies::Key;

use rtwalk::gql::ApiInfo;
use rtwalk::gql::MergedMutationRoot;
use rtwalk::gql::MergedQueryRoot;
use rtwalk::state::InnerState;
use rtwalk::state::State;

pub async fn setup(
    test_name: &str,
) -> Result<(
    Schema<MergedQueryRoot, MergedMutationRoot, EmptySubscription>,
    (
        Surreal<surrealdb::engine::remote::ws::Client>,
        Client,
        Client,
    ),
)> {
    let _ = dotenv();

    let redis = Client::connect(env::var("REDIS_URL").expect("REDIS_URL")).await?;
    let pubsub_redis = Client::connect(env::var("REDIS_URL").expect("REDIS_URL")).await?;
    let surreal_client = Surreal::new::<Ws>(env::var("DB_URL").expect("DB_URL")).await?;

    surreal_client
        .signin(Database {
            username: "root",
            password: "root",
            namespace: "test",
            database: test_name,
        })
        .await?;

    let cookies_key = env::var("COOKIE_KEY").expect("COOKIE_KEY");

    let opendal_service_builder = opendal::services::Fs::default().root("data/");

    let schema = Schema::build(
        MergedQueryRoot::default(),
        MergedMutationRoot::default(),
        EmptySubscription,
    )
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
            redis: redis.clone(),
            pubsub: pubsub_redis.clone(),
            db: surreal_client.clone(),
            op: Operator::new(opendal_service_builder)?.finish(),
            cookie_key: Key::from(cookies_key.as_bytes()),
            paseto_key: PasetoSymmetricKey::<V4, Local>::from(rusty_paseto::prelude::Key::from(
                cookies_key[..32].as_bytes(),
            )),
        }),
    })
    .finish();

    Ok((schema, (surreal_client, redis, pubsub_redis)))
}
