use std::{env, error::Error, sync::Arc};

use crate::gql::ApiInfo;

use async_graphql::{http::GraphiQLSource, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    http::{header::CONTENT_TYPE, Method},
    response::{Html, IntoResponse},
    routing::get,
    Extension, Router,
};
use cliparser::{
    help, parse_process,
    types::{Argument, ArgumentOccurrence, ArgumentValueType, CliSpec, CliSpecMetaInfo},
};
use dotenvy::dotenv;
use gql::{MergedMutationRoot, MergedQueryRoot, Subscription};
use opendal::Operator;
use rustis::client::Client;
use rusty_paseto::generic::{Local, PasetoSymmetricKey, V4};
use state::Auth;
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};
use tokio::net::TcpListener;
use tower_cookies::{CookieManagerLayer, Cookies, Key};
use tower_http::cors::CorsLayer;
use tracing::info;

pub(crate) mod config;
pub(crate) mod error;
mod gql;
pub(crate) mod models;
pub(crate) mod state;
pub(crate) mod template;

async fn graphiql() -> impl IntoResponse {
    Html(
        GraphiQLSource::build()
            .credentials(async_graphql::http::Credentials::Include)
            .endpoint("/")
            .subscription_endpoint("/ws")
            .finish(),
    )
}

async fn gql(
    schema: Extension<Schema<MergedQueryRoot, MergedMutationRoot, Subscription>>,
    cookies: Cookies,
    request: GraphQLRequest,
) -> GraphQLResponse {
    schema
        .execute(request.into_inner().data(cookies).data(Auth::default()))
        .await
        .into()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = dotenv();
    tracing_subscriber::fmt::init();
    let mut spec = CliSpec::new();

    spec = spec
        .set_meta_info(Some(CliSpecMetaInfo {
            version: Some("0.1.0".into()),
            description: Some("A simple forum backend".into()),
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

    let redis = Client::connect(env::var("REDIS_URL").expect("REDIS_URL")).await?;
    let pubsub_redis = Client::connect(env::var("REDIS_URL").expect("REDIS_URL")).await?;

    let surreal_client = Surreal::new::<Ws>(env::var("DB_URL").expect("DB_URL")).await?;

    surreal_client
        .signin(Root {
            username: "root",
            password: "root",
        })
        .await?;

    surreal_client.use_ns("dev").use_db("rtwalk").await?;

    let cookies_key = env::var("COOKIE_KEY").expect("COOKIE_KEY");

    let opendal_service_builder = opendal::services::Fs::default().root("data/");

    let schema = Schema::build(
        MergedQueryRoot::default(),
        MergedMutationRoot::default(),
        Subscription,
    )
    .data(state::State {
        inner: Arc::new(state::InnerState {
            site_name: "DreamH",
            info: ApiInfo {
                major: 0,
                minor: 1,
                bugfix: 0,
                rte: "ws://localhost:4001/ws",
                vc: "ws://localhost:4002/ws",
            },
            redis,
            pubsub: pubsub_redis,
            db: surreal_client,
            op: Operator::new(opendal_service_builder)?.finish(),
            cookie_key: Key::from(cookies_key.as_bytes()),
            paseto_key: PasetoSymmetricKey::<V4, Local>::from(rusty_paseto::prelude::Key::from(
                cookies_key[..32].as_bytes(),
            )),
        }),
    }).finish();

    let app = Router::new()
        .route("/", get(graphiql).post(gql))
        .route_service("/ws", GraphQLSubscription::new(schema.clone()))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_origin([
                    "https://dreamh.net".parse().unwrap(),
                    "http://localhost:5173".parse().unwrap(),
                ])
                .allow_credentials(true)
                .allow_headers([CONTENT_TYPE]),
        )
        .layer(CookieManagerLayer::new())
        .layer(Extension(schema));

    let port = &res.argument_values.get("port").unwrap()[0];
    let host = &res.argument_values.get("host").unwrap()[0];

    let listener = TcpListener::bind(format!("{}:{}", &host, &port,)).await?;

    info!("Starting server at {}:{}", host, port);

    drop(spec);
    drop(res);

    axum::serve(listener, app).await?;

    Ok(())
}
