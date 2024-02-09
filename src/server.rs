use std::str::FromStr;

use anyhow::anyhow;
use askama::Template;
use axum::Router;
use axum_extra::extract::Form;
use bitcoin::{
    absolute::LockTime, address::NetworkUnchecked, script::PushBytesBuf, transaction::Version,
    Address, Amount, Network, OutPoint, Psbt, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid,
    Witness,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tokio::net::TcpListener;

use crate::{ctv, error::AppError};

pub async fn server() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", axum::routing::get(index))
        .route("/locking", axum::routing::post(locking))
        .route("/spending", axum::routing::post(spending));
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
#[template(path = "locking.html")]
struct CtvTemplate {
    ctv: String,
    locking_script: String,
    locking_hex: String,
    address: String,
    addresses: Vec<Address>,
    amounts: Vec<Amount>,
}

#[derive(Deserialize)]
struct OutputsRequest {
    outputs: String,
    network: Network,
}

async fn locking(Form(request): Form<OutputsRequest>) -> Result<CtvTemplate, AppError> {
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
    let outputs: Vec<_> = addresses
        .iter()
        .zip(amounts.iter())
        .map(|(address, amount)| TxOut {
            value: *amount,
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
    let tmplhash = ctv::ctv(&tx, 0);
    let locking_script = ctv::segwit::locking_script(&tmplhash);
    let address = ctv::segwit::locking_address(&locking_script, request.network).to_string();
    Ok(CtvTemplate {
        ctv: hex::encode(tmplhash),
        locking_script: ctv::colorize(&locking_script.to_string()),
        locking_hex: hex::encode(locking_script.into_bytes()),
        address: address.to_string(),
        addresses,
        amounts,
    })
}

#[serde_as]
#[derive(Debug, Deserialize)]
struct SpendingRequest {
    ctv: String,
    addresses: Vec<Address<NetworkUnchecked>>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    amounts: Vec<Amount>,
    txid: Txid,
    vout: u32,
}

#[derive(Template)]
#[template(path = "spending.html")]
struct SpendingTemplate {
    tx: String,
}

async fn spending(Form(request): Form<SpendingRequest>) -> Result<SpendingTemplate, AppError> {
    let ctv = hex::decode(&request.ctv)?;
    let ctvpb = PushBytesBuf::try_from(ctv.clone())?;
    let script_sig = bitcoin::script::Builder::new()
        .push_slice(ctvpb)
        .into_script();
    let output: Vec<TxOut> = request
        .addresses
        .iter()
        .zip(request.amounts.iter())
        .map(|(address, amount)| TxOut {
            value: *amount,
            script_pubkey: address.clone().assume_checked().script_pubkey(),
        })
        .collect();
    let mut witness = Witness::new();
    witness.push(&ctv::segwit::locking_script(&ctv));
    let tx = Transaction {
        version: Version::ONE,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: request.txid,
                vout: request.vout,
            },
            script_sig: Default::default(),
            sequence: Sequence::ZERO,
            witness,
        }],
        output,
    };

    Ok(SpendingTemplate {
        tx: hex::encode(bitcoin::consensus::serialize(&tx)),
    })
}
