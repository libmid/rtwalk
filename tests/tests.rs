mod utils;

#[tokio::test]
async fn setup_test() {
    let _ = utils::setup().await.unwrap();
}

#[tokio::test]
async fn test_user_creation() -> anyhow::Result<()> {
    let schema = utils::setup().await?;

    Ok(())
}
