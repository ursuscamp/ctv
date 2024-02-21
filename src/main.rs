#![allow(unused)]

mod ctv;
mod error;
mod server;
mod vault;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    server::server().await?;
    Ok(())
}
