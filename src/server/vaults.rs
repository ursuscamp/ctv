use anyhow::{anyhow, bail};
use askama::Template;
use axum::Form;
use bitcoin::{
    absolute::LockTime,
    address::{NetworkChecked, NetworkUnchecked},
    transaction::Version,
    Address, Amount, Network, Sequence, Txid,
};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::{
    ctv::{
        self,
        segwit::{self, locking_address, locking_script},
        Ctv, Output,
    },
    error::AppError,
};

#[derive(Template)]
#[template(path = "vaults/index.html.jinja")]
pub(crate) struct IndexTemplate;

pub(crate) async fn index() -> IndexTemplate {
    IndexTemplate
}

#[derive(Template)]
#[template(path = "vaults/locking.html.jinja")]
pub(crate) struct LockingTemplate {
    delay: u16,
    ctv: String,
    address: Address<NetworkChecked>,
}

#[serde_as]
#[derive(Deserialize)]
pub(crate) struct LockingRequest {
    #[serde_as(as = "DisplayFromStr")]
    amount: Amount,
    cold_address: Address<NetworkUnchecked>,
    hot_address: Address<NetworkUnchecked>,
    block_delay: u16,
    network: Network,
}

pub(crate) async fn locking(
    Form(request): Form<LockingRequest>,
) -> anyhow::Result<LockingTemplate, AppError> {
    let ctv = Ctv {
        network: request.network,
        version: Version::TWO,
        locktime: LockTime::ZERO,
        sequences: vec![Sequence::from_height(request.block_delay)],
        outputs: vec![Output::Vault {
            hot: request.hot_address,
            cold: request.cold_address,
            amount: request.amount - Amount::from_sat(600),
            delay: request.block_delay,
        }],
    };
    let tmplhash = ctv.ctv()?;
    let locking_script = locking_script(&tmplhash);
    let address = locking_address(&locking_script, request.network);
    Ok(LockingTemplate {
        delay: request.block_delay,
        ctv: serde_json::to_string(&ctv)?,
        address: address.clone(),
    })
}

#[derive(Deserialize)]
pub(crate) struct UnvaultingRequest {
    ctv: String,
    delay: u16,
    txid: Txid,
    vout: u32,
}

#[derive(Template)]
#[template(path = "vaults/unvaulting.html.jinja")]
pub(crate) struct UnvaultingTemplate {
    ctv: String,
    script: String,
    tx: String,
}

pub(crate) async fn unvaulting(
    Form(request): Form<UnvaultingRequest>,
) -> anyhow::Result<UnvaultingTemplate, AppError> {
    let ctv: Ctv = serde_json::from_str(&request.ctv)?;
    let tx = ctv.spending_tx(request.txid, request.vout)?;
    let tx = hex::encode(bitcoin::consensus::serialize(&tx[0]));
    match ctv.outputs[0].clone() {
        Output::Vault {
            hot,
            cold,
            amount,
            delay,
        } => {
            let script = ctv::segwit::vault_locking_script(delay, cold, hot, ctv.network, amount)?
                .to_string();
            Ok(UnvaultingTemplate {
                ctv: request.ctv.clone(),
                script,
                tx,
            })
        }
        _ => Err(anyhow!("Invalid vault construction").into()),
    }
}

#[derive(Template)]
#[template(path = "vaults/spending.html.jinja")]
pub(crate) struct SpendingTemplate {
    cold_tx: String,
    hot_tx: String,
}

pub(crate) async fn spending() -> anyhow::Result<SpendingTemplate, AppError> {
    Ok(SpendingTemplate {
        cold_tx: String::new(),
        hot_tx: String::new(),
    })
}
