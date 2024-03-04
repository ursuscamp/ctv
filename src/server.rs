use askama::Template;
use axum::Router;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

mod simple;
mod vaults;

pub async fn server() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let app = Router::new()
        .route("/", axum::routing::get(index))
        .route("/simple", axum::routing::get(simple::index))
        .route("/simple/locking", axum::routing::post(simple::locking))
        .route("/simple/spending", axum::routing::post(simple::spending))
        .route("/vaults", axum::routing::get(vaults::index))
        .route("/vaults/locking", axum::routing::post(vaults::locking))
        .route(
            "/vaults/unvaulting",
            axum::routing::post(vaults::unvaulting),
        )
        .route("/vaults/spending", axum::routing::post(vaults::spending));
    let listener = TcpListener::bind("localhost:5555").await?;

    tracing::info!("Starting server on localhost:5555");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Template)]
#[template(path = "index.html.jinja")]
struct IndexTemplate;

async fn index() -> IndexTemplate {
    IndexTemplate
}
