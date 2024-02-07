use askama::Template;
use axum::{Form, Router};
use bitcoin::{Network, Transaction};
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
    network: Network,
}

#[derive(Template)]
#[template(path = "ctv.html")]
struct CtvTemplate {
    ctv: String,
    locking_script: String,
    locking_hex: String,
    address: String,
}

async fn ctv(Form(request): Form<CtvRequest>) -> CtvTemplate {
    let tmplhash = ctv::ctv(&request.txhash, request.input);
    let locking_script = ctv::segwit::locking_script(&tmplhash);
    let address = ctv::segwit::locking_address(&locking_script, request.network).to_string();
    CtvTemplate {
        ctv: hex::encode(tmplhash),
        locking_script: locking_script.to_string(),
        locking_hex: hex::encode(locking_script.into_bytes()),
        address: address.to_string(),
    }
}
