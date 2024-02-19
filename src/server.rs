use std::str::FromStr;

use anyhow::anyhow;
use askama::Template;
use axum::Router;
use axum_extra::extract::Form;
use bitcoin::{
    absolute::LockTime, address::NetworkUnchecked, consensus::Encodable, script::PushBytesBuf,
    transaction::Version, Address, Amount, Network, OutPoint, Psbt, ScriptBuf, Sequence,
    Transaction, TxIn, TxOut, Txid, Witness,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

mod simple;
mod vaults;

use crate::{
    ctv::{self, Ctv, Output},
    error::AppError,
};

pub async fn server() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let app = Router::new()
        .route("/", axum::routing::get(index))
        .route("/simple/locking", axum::routing::post(simple::locking))
        .route("/simple/spending", axum::routing::post(simple::spending))
        .route("/vaults", axum::routing::get(vaults::index))
        .route("/vaults/locking", axum::routing::post(vaults::locking))
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
