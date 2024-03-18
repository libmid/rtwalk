mod utils;

use async_graphql::{value, Request, Variables};
use rustis::commands::StringCommands;
use serde_json::json;

type R = anyhow::Result<()>;

#[tokio::test]
async fn setup_test() -> R {
    let _ = utils::setup("setup_test").await?;
    Ok(())
}

#[tokio::test]
async fn test_api_version() -> R {
    let (schema, _) = utils::setup("api_version_test").await?;
    let q = "{ info { major minor bugfix }}";
    let res = schema.execute(q).await;
    assert_eq!(
        res.data,
        value!({
            "info": {
                "major": 0,
                "minor": 1,
                "bugfix": 0,
            }
        })
    );
    Ok(())
}

#[tokio::test]
async fn test_user_creation() -> R {
    let (schema, (_, redis, _)) = utils::setup("test_user_creation").await?;
    let res = schema.execute(r#"mutation {
        createUser(username: "test_user_creation", email: "test@example.com", password: "sTrOnGPaSs19@!") 
    }"#).await;
    assert_eq!(
        res.data,
        value!({
            "createUser": "Verification code sent to email"
        })
    );
    let code: u64 = redis.get("verification_code:test_user_creation").await?;
    dbg!(&code);
    let r = Request::new(
        r#"
                mutation($username: String!, $code: Int!) {
                    verifyUser(username: $username, code: $code) {
                        username
                    }
                }
                "#,
    )
    .variables(Variables::from_json(json!({
        "username": "test_user_creation",
        "code": code
    })));
    let res = schema.execute(r).await;
    assert_eq!(
        res.data,
        value!({
            "verifyUser": {
            "username": "test_user_creation"
            }
        })
    );

    Ok(())
}
