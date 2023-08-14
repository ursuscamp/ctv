#![allow(unused)]

use std::str::FromStr;

use bitcoin::{
    absolute::LockTime,
    address::{WitnessProgram, WitnessVersion},
    opcodes::all::{OP_ADD, OP_EQUAL, OP_EQUALVERIFY},
    psbt::{Input, Output, PartiallySignedTransaction},
    script,
    secp256k1::Secp256k1,
    Address, Amount, Denomination, Network, OutPoint, ScriptBuf, Sequence, Transaction, TxIn,
    TxOut, Txid, Witness,
};
use ctv::sha256;
use miniscript::psbt::PsbtExt;

mod ctv;

fn main() {
    println!("{}", new_ctv_address());
    println!(
        "{}",
        hex::encode(bitcoin::consensus::serialize(&spend_tx()))
    );
}

fn new_ctv_address() -> Address {
    Address::p2wsh(&locking_script(), Network::Regtest)
}

fn locking_script() -> ScriptBuf {
    script::Builder::new()
        .push_int(1)
        .push_opcode(OP_ADD)
        .push_int(100)
        .push_opcode(OP_EQUAL)
        .into_script()
}

fn spend_tx() -> Transaction {
    let raw_tx = "020000000001019effa0606fdf7e8235b6c0d1e8f2e60e3bfcd09eb824077f4b4ff81a8b5a021e0000000000fdffffff0200e1f505000000002200208b88aeecc8810057147ae75bb5fef0b88e74cdd69877a0b0d9119cbc0a13bd183c96231801000000225120ab0af65a053979dfd08b9673405900f185b6d45edea4dddd5dec9b7ec2eba5fc01402b8aac44cbfe12c56ddfd1ff2a0021c6f9b4e76b0325b44a647365ae9e695044029f6f7eb409baee56803e80e47a643711703795c955a8bdc5a0abc5761902fece000000";
    let input_tx: Transaction =
        bitcoin::consensus::deserialize(&hex::decode(raw_tx).unwrap()).unwrap();
    let send_addr = Address::from_str("bcrt1qgd3qh7y60txnpnqfj98qy0xuudm7wzpgz5c02y")
        .unwrap()
        .require_network(Network::Regtest)
        .unwrap();

    let mut witness = Witness::new();
    witness.push(99u8.to_le_bytes());
    witness.push(&locking_script());
    let tx = Transaction {
        version: 1,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: input_tx.txid(),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ZERO,
            witness,
        }],
        output: vec![TxOut {
            value: input_tx.output[0].value / 2,
            script_pubkey: send_addr.script_pubkey(),
        }],
    };

    tx
}
