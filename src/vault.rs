use bitcoin::{
    absolute::LockTime,
    address::NetworkUnchecked,
    opcodes::all::{OP_CSV, OP_DROP, OP_ELSE, OP_ENDIF, OP_IF, OP_NOP4},
    script::PushBytesBuf,
    transaction::Version,
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid,
    Witness,
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

    pub(crate) fn cold_spend(&self, txid: Txid, vout: u32) -> anyhow::Result<Transaction> {
        let mut witness = Witness::new();
        let script = self.final_spend_script()?;
        witness.push([]);
        witness.push(script);
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
        let mut witness = Witness::new();
        let script = self.final_spend_script()?;
        witness.push([1]);
        witness.push(script);
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

    fn final_spend_address(&self) -> anyhow::Result<Address<NetworkUnchecked>> {
        Ok(Address::p2wsh(&self.final_spend_script()?, self.network)
            .as_unchecked()
            .clone())
    }

    pub(crate) fn vault_ctv(&self) -> anyhow::Result<Ctv> {
        Ok(Ctv {
            network: self.network,
            version: Version::ONE,
            locktime: LockTime::ZERO,
            sequences: vec![Sequence::ZERO],
            outputs: vec![Output::Address {
                address: self.final_spend_address()?,
                amount: self.amount - Amount::from_sat(600),
            }],
        })
    }

    fn final_spend_script(&self) -> anyhow::Result<ScriptBuf> {
        let amount = self.amount - Amount::from_sat(1200);
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

    fn cold_ctv(&self) -> anyhow::Result<Ctv> {
        Ok(Ctv {
            network: self.network,
            version: Version::ONE,
            locktime: LockTime::ZERO,
            sequences: vec![Sequence::ZERO],
            outputs: vec![Output::Address {
                address: self.cold.clone(),
                amount: self.amount - Amount::from_sat(1200),
            }],
        })
    }

    fn hot_ctv(&self) -> anyhow::Result<Ctv> {
        Ok(Ctv {
            network: self.network,
            version: Version::TWO,
            locktime: LockTime::ZERO,
            sequences: vec![Sequence::from_height(self.delay)],
            outputs: vec![Output::Address {
                address: self.hot.clone(),
                amount: self.amount - Amount::from_sat(1200),
            }],
        })
    }
}
