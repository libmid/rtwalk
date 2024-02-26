use async_graphql::{http::GraphiQLSource, EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    response::{Html, IntoResponse},
    routing::{get, post_service},
    Router,
};
use tokio::net::TcpListener;

mod gql;
pub mod models;

async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/").finish())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let schema = Schema::build(gql::QueryRoot, EmptyMutation, EmptySubscription).finish();
    let app = Router::new()
        .route("/", get(graphiql))
        .route("/api", post_service(GraphQL::new(schema)));

    let listener = TcpListener::bind("0.0.0.0:4001").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
