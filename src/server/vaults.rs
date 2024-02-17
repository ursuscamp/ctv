use askama::Template;
use axum::Form;
use bitcoin::{address::NetworkUnchecked, Address, Amount, Network};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Template)]
#[template(path = "vaults/index.html.jinja")]
pub(crate) struct IndexTemplate;

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

pub(crate) async fn index() -> IndexTemplate {
    IndexTemplate
}

pub(crate) async fn locking(Form(request): Form<LockingRequest>) -> IndexTemplate {
    IndexTemplate
}
