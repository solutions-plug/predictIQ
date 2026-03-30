#[tokio::main]
async fn main() -> anyhow::Result<()> {
    predictiq_api::run().await
}
