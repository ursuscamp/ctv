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
        .route("/locking", axum::routing::post(locking))
        .route("/spending", axum::routing::post(spending));
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
#[derive(Template)]
#[template(path = "locking.html.jinja")]
struct CtvTemplate {
    ctv_hash: String,
    locking_script: String,
    locking_hex: String,
    address: String,
    ctv: String,
}

#[derive(Debug, Deserialize)]
struct LockingRequest {
    outputs: String,
    network: Network,
    data: Option<String>,
    congestion: Option<bool>,
}

async fn locking(Form(request): Form<LockingRequest>) -> Result<CtvTemplate, AppError> {
    tracing::info!("Locking started.");
    tracing::debug!("{request:?}");
    let mut addresses = Vec::new();
    let mut amounts = Vec::new();
    for line in request.outputs.lines() {
        let (address, amount) = line
            .split_once(':')
            .ok_or_else(|| anyhow!("Incorrectly formatted output"))?;
        let address = Address::from_str(address)?.require_network(request.network)?;
        let amount = Amount::from_str(amount)?;
        addresses.push(address);
        amounts.push(amount);
    }

    let mut ctv = if request.congestion.unwrap_or_default() {
        tracing::debug!("User requested congestion control tree.");
        locking_tree(&addresses, &amounts, request.network).unwrap()
    } else {
        tracing::debug!("User requested simple CTV.");
        Ctv {
            network: request.network,
            version: Version::ONE,
            locktime: LockTime::ZERO,
            sequences: vec![Sequence::ZERO],
            outputs: addresses
                .into_iter()
                .zip(amounts.into_iter())
                .map(|(address, amount)| Output::Address {
                    address: address.as_unchecked().clone(),
                    amount: amount - Amount::from_sat(600),
                })
                .collect(),
        }
    };

    if let Some(data) = request.data {
        ctv.outputs.push(Output::Data { data });
    }

    let ctvhash = ctv.ctv()?;
    let locking_script = ctv::segwit::locking_script(&ctvhash);
    let address = ctv::segwit::locking_address(&locking_script, request.network);

    tracing::info!("Locking finished.");
    Ok(CtvTemplate {
        ctv_hash: hex::encode(ctvhash),
        locking_script: ctv::colorize(&locking_script.to_string()),
        locking_hex: hex::encode(locking_script.into_bytes()),
        address: address.to_string(),
        ctv: serde_json::to_string(&ctv)?,
    })
}

fn locking_tree(addresses: &[Address], amounts: &[Amount], network: Network) -> Option<Ctv> {
    let address = addresses.first()?.clone();
    let amount = *amounts.first()?;

    // The remaining amounts after the current output are the total amount we can send onto the next CTV
    let rem: Amount = amounts[1..].iter().copied().sum();

    // Recrusively build the locking tree
    let next_ctv = locking_tree(&addresses[1..], &amounts[1..], network);
    let mut outputs = Vec::new();
    if let Some(ctv) = next_ctv {
        outputs.push(Output::Tree {
            tree: Box::new(ctv),
            amount: rem,
        });
    }
    outputs.push(Output::Address {
        address: address.as_unchecked().clone(),
        amount: amount - Amount::from_sat(600),
    });

    Some(Ctv {
        network,
        version: Version::ONE,
        locktime: LockTime::ZERO,
        sequences: vec![Sequence::ZERO],
        outputs,
    })
}

#[derive(Debug, Deserialize)]
struct SpendingRequest {
    ctv: String,
    txid: Txid,
    vout: u32,
}

#[derive(Template)]
#[template(path = "spending.html.jinja")]
struct SpendingTemplate {
    txs: Vec<String>,
}

async fn spending(Form(request): Form<SpendingRequest>) -> Result<SpendingTemplate, AppError> {
    tracing::info!("Spending started.");
    tracing::debug!("{request:?}");
    let ctv: Ctv = serde_json::from_str(&request.ctv)?;
    let tx = ctv.spending_tx(request.txid, request.vout)?;

    tracing::info!("Spending finished.");
    Ok(SpendingTemplate {
        txs: tx
            .iter()
            .map(bitcoin::consensus::serialize)
            .map(hex::encode)
            .collect(),
    })
}
