use askama::Template;
use axum::Form;
use bitcoin::{
    absolute::LockTime,
    address::{NetworkChecked, NetworkUnchecked},
    transaction::Version,
    Address, Amount, Network, Sequence,
};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::{
    ctv::{
        segwit::{locking_address, locking_script},
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
        sequences: vec![Sequence::ZERO],
        outputs: vec![Output::Vault {
            hot: request.hot_address,
            amount: request.amount,
            delay: request.block_delay,
        }],
    };
    let tmplhash = ctv.ctv()?;
    let locking_script = locking_script(&tmplhash);
    let address = locking_address(&locking_script, request.network);
    Ok(LockingTemplate {
        address: address.clone(),
    })
}
