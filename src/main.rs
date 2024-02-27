use std::{env, ops::Deref, sync::Arc};

use crate::gql::ApiInfo;
use async_graphql::{http::GraphiQLSource, EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    response::{Html, IntoResponse},
    routing::{get, post_service},
    Router,
};
use cliparser::{
    help, parse_process,
    types::{Argument, ArgumentOccurrence, ArgumentValueType, CliSpec, CliSpecMetaInfo},
};
use mongodm::mongo;
use rustis::client::Client;
use tokio::net::TcpListener;

pub mod config;
pub mod error;
mod gql;
pub mod models;
pub mod template;
pub mod utils;

async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/").finish())
}

pub struct State {
    inner: Arc<InnerState>,
}

pub struct InnerState {
    pub site_name: &'static str,
    pub info: ApiInfo,
    pub redis: Client,
    pub pubsub: Client,
    pub mongo: mongo::Client,
}

impl Deref for State {
    type Target = InnerState;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut spec = CliSpec::new();

    spec = spec
        .set_meta_info(Some(CliSpecMetaInfo {
            version: Some("0.1.0".into()),
            description: Some("A sumple forum app".into()),
            project: Some("rtwalk".into()),
            help_post_text: None,
            author: None,
        }))
        .add_command("rtwalk")
        .add_argument(Argument {
            name: "host".into(),
            key: vec!["--host".into(), "-h".into()],
            argument_occurrence: ArgumentOccurrence::Single,
            value_type: ArgumentValueType::Single,
            default_value: Some("127.0.0.1".into()),
            help: None,
        })
        .add_argument(Argument {
            name: "port".into(),
            key: vec!["--port".into(), "-p".into()],
            argument_occurrence: ArgumentOccurrence::Single,
            value_type: ArgumentValueType::Single,
            default_value: Some("4001".into()),
            help: None,
        })
        .set_positional_argument(None);
    let res = parse_process(&spec).expect(&help(&spec));

    let redis = Client::connect("127.0.0.1:6379")
        .await
        .expect("Redis connection failed");
    let pubsub_redis = Client::connect("127.0.0.1:6379")
        .await
        .expect("Redis connection failed");
    let mongo_client =
        mongo::Client::with_uri_str(&env::var("MONGODB_URL").expect("MONGODB_URL not set"))
            .await
            .expect("Mongodb connection failed");

    let schema = Schema::build(gql::QueryRoot, EmptyMutation, EmptySubscription)
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
                mongo: mongo_client.clone(),
            }),
        })
        .finish();

    let app = Router::new()
        .route("/", get(graphiql))
        .route("/api", post_service(GraphQL::new(schema)));

    let listener = TcpListener::bind(format!(
        "{}:{}",
        res.argument_values.get("host").unwrap()[0],
        res.argument_values.get("port").unwrap()[0]
    ))
    .await?;
    axum::serve(listener, app).await?;

    mongo_client.shutdown().await;

    Ok(())
}
