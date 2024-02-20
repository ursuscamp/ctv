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
    unvault_ctv: String,
    hot_ctv: String,
    cold_ctv: String,
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
    let unvault_amount = request.amount - Amount::from_sat(600);
    let unvault_ctv = Ctv {
        network: request.network,
        version: Version::TWO,
        locktime: LockTime::ZERO,
        sequences: vec![Sequence::from_height(request.block_delay)],
        outputs: vec![Output::Vault {
            hot: request.hot_address.clone(),
            cold: request.cold_address.clone(),
            amount: request.amount - Amount::from_sat(600),
            delay: request.block_delay,
        }],
    };
    let spend_amount = unvault_amount - Amount::from_sat(600);
    let hot_ctv = Ctv {
        network: request.network,
        version: Version::ONE,
        locktime: LockTime::ZERO,
        sequences: vec![Sequence::ZERO],
        outputs: vec![Output::Address {
            address: request.hot_address.clone(),
            amount: spend_amount,
        }],
    };

    let mut cold_ctv = hot_ctv.clone();
    cold_ctv.outputs[0] = Output::Address {
        address: request.cold_address.clone(),
        amount: spend_amount,
    };
    let unvault_tmplhash = unvault_ctv.ctv()?;
    let locking_script = locking_script(&unvault_tmplhash);
    let address = locking_address(&locking_script, request.network);
    Ok(LockingTemplate {
        delay: request.block_delay,
        unvault_ctv: serde_json::to_string(&unvault_ctv)?,
        hot_ctv: serde_json::to_string(&hot_ctv)?,
        cold_ctv: serde_json::to_string(&cold_ctv)?,
        address: address.clone(),
    })
}

#[derive(Deserialize)]
pub(crate) struct UnvaultingRequest {
    unvault_ctv: String,
    hot_ctv: String,
    cold_ctv: String,
    delay: u16,
    txid: Txid,
    vout: u32,
}

#[derive(Template)]
#[template(path = "vaults/unvaulting.html.jinja")]
pub(crate) struct UnvaultingTemplate {
    unvault_ctv: String,
    hot_ctv: String,
    cold_ctv: String,
    script: String,
    tx: String,
}

pub(crate) async fn unvaulting(
    Form(request): Form<UnvaultingRequest>,
) -> anyhow::Result<UnvaultingTemplate, AppError> {
    let ctv: Ctv = serde_json::from_str(&request.unvault_ctv)?;
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
                unvault_ctv: request.unvault_ctv.clone(),
                hot_ctv: request.hot_ctv,
                cold_ctv: request.cold_ctv,
                script,
                tx,
            })
        }
        _ => Err(anyhow!("Invalid vault construction").into()),
    }
}

#[derive(Deserialize)]
pub(crate) struct SpendingRequest {
    unvault_ctv: String,
    hot_ctv: String,
    cold_ctv: String,
    txid: Txid,
    vout: u32,
}

#[derive(Template)]
#[template(path = "vaults/spending.html.jinja")]
pub(crate) struct SpendingTemplate {
    cold_tx: String,
    hot_tx: String,
}

pub(crate) async fn spending(
    Form(request): Form<SpendingRequest>,
) -> anyhow::Result<SpendingTemplate, AppError> {
    let hot_ctv: Ctv = serde_json::from_str(&request.hot_ctv)?;
    let hot_tx = hot_ctv.spending_tx(request.txid, request.vout)?;
    let cold_ctv: Ctv = serde_json::from_str(&request.cold_ctv)?;
    let cold_tx = cold_ctv.spending_tx(request.txid, request.vout)?;
    Ok(SpendingTemplate {
        cold_tx: hex::encode(bitcoin::consensus::serialize(&cold_tx)),
        hot_tx: hex::encode(bitcoin::consensus::serialize(&hot_tx)),
    })
}
