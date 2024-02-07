use askama::Template;
use axum::{Form, Router};
use bitcoin::Transaction;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::ctv;

pub async fn server() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", axum::routing::get(index))
        .route("/ctv", axum::routing::post(ctv));
    let listener = TcpListener::bind("0.0.0.0:5555").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

async fn index() -> IndexTemplate {
    IndexTemplate
}

#[derive(Deserialize)]
struct CtvRequest {
    #[serde(with = "bitcoin::consensus::serde::With::<bitcoin::consensus::serde::Hex>")]
    txhash: Transaction,

    input: u32,
}

#[derive(Template)]
#[template(path = "ctv.html")]
struct CtvTemplate {
    ctv: String,
}

async fn ctv(Form(request): Form<CtvRequest>) -> CtvTemplate {
    let tmplhash = ctv::ctv(&request.txhash, request.input);
    let tmplhash = hex::encode(tmplhash);
    CtvTemplate { ctv: tmplhash }
}
