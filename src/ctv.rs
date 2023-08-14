use std::io::{Cursor, Write};

use bitcoin::{consensus::Encodable, Transaction};
use sha2::{Digest, Sha256};

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
                bitcoin::consensus::deserialize(&hex::decode(&hex_tx).unwrap()).unwrap();
            let spend_index = td["spend_index"]
                .as_array()
                .unwrap()
                .into_iter()
                .map(|i| i.as_i64().unwrap())
                .collect::<Vec<i64>>();
            let result: Vec<String> = td["result"]
                .as_array()
                .unwrap()
                .into_iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect();

            for (idx, si) in spend_index.into_iter().enumerate() {
                let hash = hex::encode(ctv(&tx, si as u32));
                assert_eq!(hash, result[idx]);
            }
        }
    }
}
