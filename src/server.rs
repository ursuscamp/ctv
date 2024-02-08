use std::str::FromStr;

use askama::Template;
use axum::{Form, Router};
use bitcoin::{
    absolute::LockTime, transaction::Version, Address, Amount, Network, Sequence, Transaction,
    TxIn, TxOut,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::ctv;

pub async fn server() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", axum::routing::get(index))
        .route("/outputs", axum::routing::post(outputs));
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
#[derive(Template)]
#[template(path = "ctv.html")]
struct CtvTemplate {
    ctv: String,
    locking_script: String,
    locking_hex: String,
    address: String,
}

#[derive(Deserialize)]
struct OutputsRequest {
    outputs: String,
    input: u32,
    network: Network,
}

async fn outputs(Form(request): Form<OutputsRequest>) -> CtvTemplate {
    let outputs: Vec<_> = request
        .outputs
        .lines()
        .map(|line| {
            let (address, amount) = line.split_once(':').unwrap();
            let address = Address::from_str(address)
                .unwrap()
                .require_network(request.network)
                .unwrap();
            let amount = Amount::from_str(amount).unwrap();
            (address, amount)
        })
        .map(|(address, amount)| TxOut {
            value: amount,
            script_pubkey: address.script_pubkey(),
        })
        .collect();
    let tx = Transaction {
        version: Version::ONE,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            sequence: Sequence::ZERO,
            ..Default::default()
        }],
        output: outputs,
    };
    let tmplhash = ctv::ctv(&tx, request.input);
    let locking_script = ctv::segwit::locking_script(&tmplhash);
    let address = ctv::segwit::locking_address(&locking_script, request.network).to_string();
    CtvTemplate {
        ctv: hex::encode(tmplhash),
        locking_script: locking_script.to_string(),
        locking_hex: hex::encode(locking_script.into_bytes()),
        address: address.to_string(),
    }
}
