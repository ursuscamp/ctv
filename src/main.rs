mod error;
mod server;
mod util;
mod vault;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    server::server().await?;
    Ok(())
}
