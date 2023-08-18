#![allow(unused)]

mod ctv; // 1
mod p2wsh; // 2
mod segwit_ctv; // 3
mod taproot_ctv; //4

fn main() {
    taproot_ctv::run();
}
