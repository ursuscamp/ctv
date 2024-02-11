use std::io::{Cursor, Write};

use bitcoin::{
    absolute::LockTime,
    address::{NetworkChecked, NetworkUnchecked},
    consensus::Encodable,
    transaction::Version,
    Address, Amount, Network, ScriptBuf, Sequence, Transaction, TxIn, TxOut,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ctv {
    network: Network,
    version: Version,
    locktime: LockTime,
    scripts_sigs: Vec<ScriptBuf>,
    sequences: Vec<Sequence>,
    outputs: Vec<Output>,
    input_index: u32,
}

impl Ctv {
    pub fn as_tx(&self) -> anyhow::Result<Transaction> {
        let input = self
            .sequences
            .iter()
            .map(|seq| TxIn {
                sequence: *seq,
                ..Default::default()
            })
            .collect();
        let output: anyhow::Result<Vec<TxOut>> = self
            .outputs
            .iter()
            .map(|output| output.as_txout(self.network))
            .collect();
        Ok(Transaction {
            version: self.version,
            lock_time: self.locktime,
            input,
            output: output?,
        })
    }

    pub fn ctv(&self) -> anyhow::Result<Vec<u8>> {
        Ok(ctv(&self.as_tx()?, self.input_index))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Output {
    Address {
        address: Address<NetworkUnchecked>,
        amount: Amount,
    },
}

impl Output {
    pub fn as_txout(&self, network: Network) -> anyhow::Result<TxOut> {
        Ok(match self {
            Output::Address { address, amount } => TxOut {
                value: *amount,
                script_pubkey: address.clone().require_network(network)?.script_pubkey(),
            },
        })
    }
}

pub fn ctv(tx: &Transaction, input: u32) -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::<u8>::new());
    tx.version.consensus_encode(&mut buffer).unwrap();
    tx.lock_time.consensus_encode(&mut buffer).unwrap();
    if let Some(scriptsigs) = scriptsigs(tx) {
        buffer.write_all(&scriptsigs).unwrap();
    }
    (tx.input.len() as u32)
        .consensus_encode(&mut buffer)
        .unwrap();
    buffer.write_all(&sequences(tx)).unwrap();
    (tx.output.len() as u32)
        .consensus_encode(&mut buffer)
        .unwrap();
    buffer.write_all(&outputs(tx)).unwrap();
    input.consensus_encode(&mut buffer).unwrap();
    let buffer = buffer.into_inner();
    sha256(buffer)
}

fn scriptsigs(tx: &Transaction) -> Option<Vec<u8>> {
    // If there are no scripts sigs, do nothing
    if tx.input.iter().all(|txin| txin.script_sig.is_empty()) {
        return None;
    }

    let scripts_sigs = tx
        .input
        .iter()
        .fold(Cursor::new(Vec::new()), |mut cursor, txin| {
            txin.script_sig.consensus_encode(&mut cursor).unwrap();
            cursor
        })
        .into_inner();
    Some(sha256(scripts_sigs))
}

fn sequences(tx: &Transaction) -> Vec<u8> {
    let sequences = tx
        .input
        .iter()
        .fold(Cursor::new(Vec::new()), |mut cursor, txin| {
            txin.sequence.consensus_encode(&mut cursor).unwrap();
            cursor
        })
        .into_inner();
    sha256(sequences)
}

fn outputs(tx: &Transaction) -> Vec<u8> {
    let outputs = tx
        .output
        .iter()
        .fold(Cursor::new(Vec::new()), |mut cursor, txout| {
            txout.consensus_encode(&mut cursor).unwrap();
            cursor
        })
        .into_inner();
    sha256(outputs)
}

pub fn sha256(data: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

pub fn colorize(script: &str) -> String {
    let opcode = Regex::new(r"(OP_\w+)").unwrap();
    let hex = Regex::new(r"([0-9a-z]{64})").unwrap();
    let color = opcode.replace_all(script, r#"<span style="color: red">$1</span>"#);
    let color = hex.replace_all(&color, r#"<span style="color: green">$1</span>"#);

    color.replace("OP_NOP4", "OP_CTV")
}

pub mod segwit {
    use bitcoin::{opcodes::all::OP_NOP4, Address, Network, Script, ScriptBuf};

    pub fn locking_address(script: &Script, network: Network) -> Address {
        Address::p2wsh(script, network)
    }

    pub fn locking_script(tmplhash: &[u8]) -> ScriptBuf {
        let bytes = <&[u8; 32]>::try_from(tmplhash).unwrap();
        bitcoin::script::Builder::new()
            .push_slice(bytes)
            .push_opcode(OP_NOP4)
            .into_script()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn test_ctv() {
        let test_data = include_str!("../tests/ctvhash.json");
        let test_data: Vec<Value> = serde_json::from_str(test_data).unwrap();
        for td in test_data {
            if td.is_string() {
                continue;
            }
            let td = td.as_object().unwrap();
            let hex_tx = td["hex_tx"].as_str().unwrap();
            let tx: Transaction =
                bitcoin::consensus::deserialize(&hex::decode(hex_tx).unwrap()).unwrap();
            let spend_index = td["spend_index"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i.as_i64().unwrap())
                .collect::<Vec<i64>>();
            let result: Vec<String> = td["result"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect();

            for (idx, si) in spend_index.into_iter().enumerate() {
                let hash = hex::encode(ctv(&tx, si as u32));
                assert_eq!(hash, result[idx]);
            }
        }
    }
}
