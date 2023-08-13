#![allow(unused)]

use std::io::{Cursor, Write};

use bitcoin::{consensus::Encodable, Transaction};
use sha2::{Digest, Sha256};

fn main() {
    println!("{}", 3116999548i64 as u32);
}

fn ctv(tx: &Transaction, input: u32) -> Vec<u8> {
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

fn sha256(data: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctv() {
        let tx = include_str!("../tests/tx.txt");
        let inputs = [0i64, 1, 3116999548, 4294967295]
            .into_iter()
            .map(|s| s as u32)
            .collect::<Vec<_>>();
        let results = [
            "2d28d0672f1d46cb3e86abd7e682d2d3e9961e6c9237157f47d39f0a694bb694",
            "12f7ab0a282fb9e29c9fd2ada21f950f492bfd5778a94202398c13ae6e97f0b4",
            "0ee9cc212182845d4c32ba6b3ba8859800d5cf423c58fb1444feaf21aa9cf81c",
            "da78ece7c0888725532355018961f58ad471f242e29a60adf84c55007fad608f",
        ];

        let tx = hex::decode(&tx).unwrap();
        let tx: Transaction = bitcoin::consensus::deserialize(&tx).unwrap();
        for (idx, input) in inputs.iter().enumerate() {
            let calculated = hex::encode(ctv(&tx, *input));
            assert_eq!(calculated, results[idx]);
        }
    }
}
