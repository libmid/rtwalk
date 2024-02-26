use async_graphql::{Context, Object, SimpleObject};

pub struct QueryRoot;

#[derive(SimpleObject)]
struct ApiInfo {
    major: u16,
    minor: u16,
    bugfix: u16,
    rte: &'static str,
    vc: &'static str,
}

#[Object]
impl QueryRoot {
    async fn info(&self, _ctx: &Context<'_>) -> ApiInfo {
        ApiInfo {
            major: 0,
            minor: 1,
            bugfix: 0,
            rte: "ws://localhost:4001/rte",
            vc: "ws://localhost:4002/ws",
        }
    }
}
