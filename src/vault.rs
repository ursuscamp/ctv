use bitcoin::{
    absolute::LockTime, address::NetworkUnchecked, transaction::Version, Address, Amount, Network,
    ScriptBuf, Sequence,
};
use serde::{Deserialize, Serialize};

use crate::ctv::{self, Ctv, Output};

#[derive(Serialize, Deserialize)]
pub(crate) struct Vault {
    pub(crate) hot: Address<NetworkUnchecked>,
    pub(crate) cold: Address<NetworkUnchecked>,
    pub(crate) amount: Amount,
    pub(crate) network: Network,
    pub(crate) delay: u16,
}

impl Vault {
    pub(crate) fn vault_address(&self) -> anyhow::Result<Address<NetworkUnchecked>> {
        let vault_ctv = self.vault_ctv()?;
        let vault_hash = vault_ctv.ctv()?;
        let vault_lock = ctv::segwit::locking_script(&vault_hash);
        Ok(ctv::segwit::locking_address(&vault_lock, self.network)
            .as_unchecked()
            .clone())
    }

    pub(crate) fn to_ctv(&self) -> anyhow::Result<Ctv> {
        let script = self.final_spend_script()?;
        todo!()
    }

    fn final_spend_script(&self) -> anyhow::Result<ScriptBuf> {
        crate::ctv::segwit::vault_locking_script(
            self.delay,
            self.cold.clone(),
            self.hot.clone(),
            self.network,
            self.amount,
        )
    }

    fn final_spend_address(&self) -> anyhow::Result<Address<NetworkUnchecked>> {
        Ok(Address::p2wsh(&self.final_spend_script()?, self.network)
            .as_unchecked()
            .clone())
    }

    fn unvault_ctv(&self) -> anyhow::Result<Ctv> {
        Ok(Ctv {
            network: self.network,
            version: Version::TWO,
            locktime: LockTime::ZERO,
            sequences: vec![Sequence::from_height(self.delay)],
            outputs: vec![Output::Address {
                address: self.final_spend_address()?,
                amount: self.amount - Amount::from_sat(1200),
            }],
        })
    }

    pub(crate) fn vault_ctv(&self) -> anyhow::Result<Ctv> {
        Ok(Ctv {
            network: self.network,
            version: Version::ONE,
            locktime: LockTime::ZERO,
            sequences: vec![Sequence::ZERO],
            outputs: vec![Output::Tree {
                tree: Box::new(self.unvault_ctv()?),
                amount: self.amount - Amount::from_sat(600),
            }],
        })
    }
}
