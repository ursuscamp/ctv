use std::str::FromStr;

use bitcoin::{
    absolute::LockTime, opcodes::all::OP_NOP4, script, Address, Amount, Network, OutPoint,
    ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};

use crate::ctv;

pub fn run() {
    println!("Locking address: {}", locking_address());
    println!(
        "Failing spend: {}",
        hex::encode(bitcoin::consensus::serialize(&failing_spend()))
    );
    println!(
        "Valid spend: {}",
        hex::encode(bitcoin::consensus::serialize(&valid_spend()))
    );
}

static COV_TX: &'static str = "094a05b1ded147dd5793c39a3466e43c4acf8318900e1c6caab52f0fb9bc53c3"; // After you have locking address, put the tx here
static VALID_ARR: &'static str = "bcrt1q6cwhskxlt8ptl982xz4djcggtt0260208wstzs"; // This is the address the covenant should use to pass
static FAIL_ADDR: &'static str = "bcrt1q8wl6lea80vhrg70sh22grnfctqc5kz2dgtvkwr"; // This is an address to send to which should FAIL the covenant
static VOUT: &'static u32 = &1;

// This is intended to pass
fn valid_spend() -> Transaction {
    let mut tx = failing_spend();
    tx.output[0].script_pubkey = Address::from_str(VALID_ARR)
        .unwrap()
        .assume_checked()
        .script_pubkey();
    tx
}

// This is intended to fail
fn failing_spend() -> Transaction {
    let mut witness = Witness::new();
    witness.push(&locking_script());
    Transaction {
        version: 1,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: Txid::from_str(COV_TX).unwrap(),
                vout: *VOUT,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ZERO,
            witness,
        }],
        output: vec![TxOut {
            value: Amount::from_btc(1f64).unwrap().to_sat(),
            script_pubkey: Address::from_str(FAIL_ADDR)
                .unwrap()
                .assume_checked()
                .script_pubkey(),
        }],
    }
}

fn locking_address() -> Address {
    Address::p2wsh(&locking_script(), Network::Regtest)
}

fn locking_script() -> ScriptBuf {
    script::Builder::new()
        .push_slice(<&[u8; 32]>::try_from(ctv_hash().as_slice()).unwrap())
        .push_opcode(OP_NOP4)
        .into_script()
}

fn ctv_hash() -> Vec<u8> {
    ctv::ctv(&covenant_tx(), 0)
}

fn covenant_tx() -> Transaction {
    let spk = Address::from_str(VALID_ARR)
        .unwrap()
        .assume_checked()
        .script_pubkey();

    Transaction {
        version: 1,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            sequence: Sequence::ZERO,
            ..Default::default()
        }],
        output: vec![TxOut {
            value: Amount::from_btc(1f64).unwrap().to_sat(),
            script_pubkey: spk,
        }],
    }
}
