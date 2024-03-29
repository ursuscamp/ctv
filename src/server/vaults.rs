use askama::Template;
use axum::Form;
use bitcoin::{
    address::{NetworkChecked, NetworkUnchecked},
    Address, Amount, Network, Txid,
};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::{
    error::AppError,
    util::{self},
    vault::Vault,
};

// INITIATE A VAULT
// -------------------

#[derive(Template)]
#[template(path = "vaults/index.html.jinja")]
pub(crate) struct IndexTemplate;

pub(crate) async fn index() -> IndexTemplate {
    IndexTemplate
}

// VAULTING FUNDS
// -------------------

#[derive(Template)]
#[template(path = "vaults/vaulting.html.jinja")]
pub(crate) struct VaultingTemplate {
    vault: String,
    address: Address<NetworkChecked>,
}

#[serde_as]
#[derive(Deserialize)]
pub(crate) struct VaultingRequest {
    #[serde_as(as = "DisplayFromStr")]
    amount: Amount,
    cold_address: Address<NetworkUnchecked>,
    hot_address: Address<NetworkUnchecked>,
    block_delay: u16,
    network: Network,
    taproot: Option<bool>,
}

impl From<VaultingRequest> for Vault {
    fn from(value: VaultingRequest) -> Self {
        Vault {
            hot: value.hot_address,
            cold: value.cold_address,
            amount: value.amount,
            network: value.network,
            delay: value.block_delay,
            taproot: value.taproot.unwrap_or_default(),
        }
    }
}

pub(crate) async fn vaulting(
    Form(request): Form<VaultingRequest>,
) -> anyhow::Result<VaultingTemplate, AppError> {
    let vault: Vault = request.into();
    let address = vault.vault_address()?.require_network(vault.network)?;
    let vault = serde_json::to_string(&vault)?;
    Ok(VaultingTemplate { vault, address })
}

// UNVAULTING FUNDS
// -------------------

#[derive(Deserialize)]
pub(crate) struct UnvaultingRequest {
    vault: String,
    txid: Txid,
    vout: u32,
}

#[derive(Template)]
#[template(path = "vaults/unvaulting.html.jinja")]
pub(crate) struct UnvaultingTemplate {
    vault: String,
    script: String,
    tx: String,
    txid: Txid,
}

pub(crate) async fn unvaulting(
    Form(request): Form<UnvaultingRequest>,
) -> anyhow::Result<UnvaultingTemplate, AppError> {
    let vault: Vault = serde_json::from_str(&request.vault)?;
    let script = vault.unvault_redeem_script()?;
    let script = util::colorize(&script.to_string());
    let vault_ctv = vault.vault_ctv()?;
    let spending_tx = vault_ctv.spending_tx(request.txid, request.vout)?[0].clone();
    let tx = hex::encode(bitcoin::consensus::serialize(&spending_tx));
    let vault = serde_json::to_string(&vault)?;
    Ok(UnvaultingTemplate {
        vault,
        script,
        tx,
        txid: spending_tx.txid(),
    })
}

#[derive(Deserialize)]
pub(crate) struct SpendingRequest {
    vault: String,
    txid: Txid,
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
    let vault: Vault = serde_json::from_str(&request.vault)?;
    let cold_tx = vault.cold_spend(request.txid, 0)?;
    let hot_tx = vault.hot_spend(request.txid, 0)?;
    Ok(SpendingTemplate {
        cold_tx: hex::encode(bitcoin::consensus::serialize(&cold_tx)),
        hot_tx: hex::encode(bitcoin::consensus::serialize(&hot_tx)),
    })
}
