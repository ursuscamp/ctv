use std::str::FromStr;

use anyhow::anyhow;
use askama::Template;
use axum::Form;
use bitcoin::{
    absolute::LockTime, transaction::Version, Address, Amount, Network, Sequence, Txid,
    XOnlyPublicKey,
};
use ctvlib::{Context, Fields, Output, TxType};
use serde::Deserialize;

use crate::{error::AppError, util};

#[derive(Template)]
#[template(path = "simple/index.html.jinja")]
pub(crate) struct IndexTemplate;

pub(crate) async fn index() -> IndexTemplate {
    IndexTemplate
}

#[derive(Template)]
#[template(path = "simple/locking.html.jinja")]
pub(crate) struct ContextTemplate {
    ctv_hash: String,
    locking_script: String,
    locking_hex: String,
    address: String,
    ctv: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LockingRequest {
    outputs: String,
    network: Network,
    congestion: Option<bool>,
    taproot: Option<bool>,
}

pub(crate) async fn locking(
    Form(request): Form<LockingRequest>,
) -> Result<ContextTemplate, AppError> {
    tracing::info!("Locking started.");
    tracing::debug!("{request:?}");
    let ctv = extract_ctv_from_request(&request)?;

    let ctvhash = ctv.ctv()?;
    let locking_script = ctv.locking_script()?;
    let address = ctv.address()?;

    tracing::info!("Locking finished.");
    Ok(ContextTemplate {
        ctv_hash: hex::encode(ctvhash),
        locking_script: util::colorize(&locking_script.to_string()),
        locking_hex: hex::encode(locking_script.into_bytes()),
        address: address.to_string(),
        ctv: serde_json::to_string(&ctv)?,
    })
}

fn extract_ctv_from_request(request: &LockingRequest) -> Result<Context, AppError> {
    let mut addresses = Vec::new();
    let mut amounts = Vec::new();
    let mut datas = Vec::new();
    for line in request.outputs.lines() {
        let mut splitter = line.split(':');
        let address =
            Address::from_str(splitter.next().ok_or_else(|| anyhow!("Missing address"))?)?
                .require_network(request.network)?;
        let amount = Amount::from_str(splitter.next().ok_or_else(|| anyhow!("Missing amount"))?)?;
        addresses.push(address);
        amounts.push(amount);
        datas.push(splitter.next().map(ToString::to_string));
    }
    let tx_type = if request.taproot.unwrap_or_default() {
        TxType::Taproot {
            internal_key: nums_points(),
        }
    } else {
        TxType::Segwit
    };
    let ctv = if request.congestion.unwrap_or_default() {
        tracing::debug!("User requested congestion control tree.");
        locking_tree(&addresses, &amounts, &datas, request.network, tx_type).unwrap()
    } else {
        tracing::debug!("User requested simple CTV.");
        simple_ctv(addresses, amounts, datas, request, tx_type)
    };
    Ok(ctv)
}

fn nums_points() -> XOnlyPublicKey {
    ctvlib::util::hash2curve(b"Activate CTV now!")
}

fn simple_ctv(
    addresses: Vec<Address>,
    amounts: Vec<Amount>,
    datas: Vec<Option<String>>,
    request: &LockingRequest,
    tx_type: TxType,
) -> Context {
    let mut outputs = Vec::new();
    for ((address, amount), data) in addresses
        .into_iter()
        .zip(amounts.into_iter())
        .zip(datas.into_iter())
    {
        outputs.push(Output::Address {
            address: address.as_unchecked().clone(),
            amount: amount - Amount::from_sat(600),
        });
        if let Some(data) = data {
            outputs.push(Output::Data { data });
        }
    }
    Context {
        network: request.network,
        tx_type,
        fields: Fields {
            version: Version::ONE,
            locktime: LockTime::ZERO,
            sequences: vec![Sequence::ZERO],
            outputs,
            input_idx: 0,
        },
    }
}

fn locking_tree(
    addresses: &[Address],
    amounts: &[Amount],
    datas: &[Option<String>],
    network: Network,
    tx_type: TxType,
) -> Option<Context> {
    let address = addresses.first()?.clone();
    let amount = *amounts.first()?;
    let data = datas.first()?;

    // The remaining amounts after the current output are the total amount we can send onto the next CTV
    let rem: Amount = amounts[1..].iter().copied().sum();

    // Recrusively build the locking tree
    let next_ctv = locking_tree(
        &addresses[1..],
        &amounts[1..],
        &datas[1..],
        network,
        tx_type,
    );
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

    if let Some(data) = data {
        outputs.push(Output::Data { data: data.clone() });
    }

    Some(Context {
        network,
        tx_type,
        fields: Fields {
            version: Version::ONE,
            locktime: LockTime::ZERO,
            sequences: vec![Sequence::ZERO],
            outputs,
            input_idx: 0,
        },
    })
}

#[derive(Debug, Deserialize)]
pub(crate) struct SpendingRequest {
    ctv: String,
    txid: Txid,
    vout: u32,
}

#[derive(Template)]
#[template(path = "simple/spending.html.jinja")]
pub(crate) struct SpendingTemplate {
    txs: Vec<String>,
}

pub(crate) async fn spending(
    Form(request): Form<SpendingRequest>,
) -> Result<SpendingTemplate, AppError> {
    tracing::info!("Spending started.");
    tracing::debug!("{request:?}");
    let ctv: Context = serde_json::from_str(&request.ctv)?;
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
