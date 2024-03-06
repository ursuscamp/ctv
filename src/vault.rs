use anyhow::anyhow;
use bitcoin::{
    absolute::LockTime,
    address::NetworkUnchecked,
    opcodes::all::{OP_CSV, OP_DROP, OP_ELSE, OP_ENDIF, OP_IF, OP_NOP4},
    script::PushBytesBuf,
    secp256k1::SECP256K1,
    taproot::{LeafVersion, TaprootBuilder, TaprootSpendInfo},
    transaction::Version,
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid,
    Witness, XOnlyPublicKey,
};
use ctvlib::{Context, Fields, Output, TxType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Vault {
    pub(crate) hot: Address<NetworkUnchecked>,
    pub(crate) cold: Address<NetworkUnchecked>,
    pub(crate) amount: Amount,
    pub(crate) network: Network,
    pub(crate) delay: u16,
    pub(crate) taproot: bool,
}

impl Vault {
    pub(crate) fn vault_address(&self) -> anyhow::Result<Address<NetworkUnchecked>> {
        let vault_ctv = self.vault_ctv()?;
        Ok(vault_ctv.address().map(|a| a.as_unchecked().clone())?)
    }

    pub(crate) fn cold_spend(&self, txid: Txid, vout: u32) -> anyhow::Result<Transaction> {
        let witness = self.witness(false)?;
        Ok(Transaction {
            version: Version::ONE,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint { txid, vout },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::ZERO,
                witness,
            }],
            output: vec![TxOut {
                value: self.amount - Amount::from_sat(1200),
                script_pubkey: self.cold.clone().assume_checked().script_pubkey(),
            }],
        })
    }

    pub(crate) fn hot_spend(&self, txid: Txid, vout: u32) -> anyhow::Result<Transaction> {
        let witness = self.witness(true)?;
        Ok(Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint { txid, vout },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::from_height(self.delay),
                witness,
            }],
            output: vec![TxOut {
                value: self.amount - Amount::from_sat(1200),
                script_pubkey: self.hot.clone().assume_checked().script_pubkey(),
            }],
        })
    }

    pub(crate) fn vault_ctv(&self) -> anyhow::Result<Context> {
        Ok(Context {
            network: self.network,
            tx_type: self.tx_type(),
            fields: Fields {
                version: Version::ONE,
                locktime: LockTime::ZERO,
                sequences: vec![Sequence::ZERO],
                outputs: vec![Output::Address {
                    address: self.unvault_address()?,
                    amount: self.amount - Amount::from_sat(600),
                }],
                input_idx: 0,
            },
        })
    }

    pub(crate) fn unvault_redeem_script(&self) -> anyhow::Result<ScriptBuf> {
        let _amount = self.amount - Amount::from_sat(1200);
        let cold_ctv = self.cold_ctv()?;
        let cold_hash = PushBytesBuf::try_from(cold_ctv.ctv()?)?;
        let hot_ctv = self.hot_ctv()?;
        let hot_hash = PushBytesBuf::try_from(hot_ctv.ctv()?)?;
        Ok(bitcoin::script::Builder::new()
            .push_opcode(OP_IF)
            .push_sequence(Sequence::from_height(self.delay))
            .push_opcode(OP_CSV)
            .push_opcode(OP_DROP)
            .push_slice(hot_hash)
            .push_opcode(OP_NOP4)
            .push_opcode(OP_ELSE)
            .push_slice(cold_hash)
            .push_opcode(OP_NOP4)
            .push_opcode(OP_ENDIF)
            .into_script())
    }

    fn cold_ctv(&self) -> anyhow::Result<Context> {
        Ok(Context {
            network: self.network,
            tx_type: self.tx_type(),
            fields: Fields {
                version: Version::ONE,
                locktime: LockTime::ZERO,
                sequences: vec![Sequence::ZERO],
                outputs: vec![Output::Address {
                    address: self.cold.clone(),
                    amount: self.amount - Amount::from_sat(1200),
                }],
                input_idx: 0,
            },
        })
    }

    fn hot_ctv(&self) -> anyhow::Result<Context> {
        Ok(Context {
            network: self.network,
            tx_type: self.tx_type(),
            fields: Fields {
                version: Version::TWO,
                locktime: LockTime::ZERO,
                sequences: vec![Sequence::from_height(self.delay)],
                outputs: vec![Output::Address {
                    address: self.hot.clone(),
                    amount: self.amount - Amount::from_sat(1200),
                }],
                input_idx: 0,
            },
        })
    }

    fn unvault_address(&self) -> anyhow::Result<Address<NetworkUnchecked>> {
        match self.tx_type() {
            TxType::Segwit => Ok(Address::p2wsh(&self.unvault_redeem_script()?, self.network)
                .as_unchecked()
                .clone()),
            TxType::Taproot { internal_key } => {
                let tsi = self.unvault_taproot_spend_info(internal_key)?;
                Ok(
                    Address::p2tr(SECP256K1, internal_key, tsi.merkle_root(), self.network)
                        .as_unchecked()
                        .clone(),
                )
            }
        }
    }

    fn tx_type(&self) -> TxType {
        if self.taproot {
            return TxType::Taproot {
                internal_key: nums_points(),
            };
        }
        TxType::Segwit
    }

    fn unvault_taproot_spend_info(
        &self,
        internal_key: XOnlyPublicKey,
    ) -> anyhow::Result<TaprootSpendInfo> {
        TaprootBuilder::new()
            .add_leaf(0, self.unvault_redeem_script()?)?
            .finalize(SECP256K1, internal_key)
            .map_err(|_| anyhow!("Taproot not finalizable"))
    }

    fn witness(&self, hot: bool) -> anyhow::Result<Witness> {
        let rs = self.unvault_redeem_script()?;
        let mut witness = Witness::new();
        if hot {
            witness.push([1]);
        } else {
            witness.push([]);
        }

        witness.push(rs.clone());
        match self.tx_type() {
            TxType::Segwit => {}
            TxType::Taproot { internal_key } => {
                let tsi = self.unvault_taproot_spend_info(internal_key)?;
                let cb = tsi
                    .control_block(&(rs, LeafVersion::TapScript))
                    .ok_or_else(|| anyhow!("Invalid tapscript formation"))?;
                witness.push(cb.serialize());
            }
        }
        Ok(witness)
    }
}

fn nums_points() -> XOnlyPublicKey {
    ctvlib::util::hash2curve(b"Activate CTV now!")
}
