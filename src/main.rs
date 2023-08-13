use bitcoin::Transaction;

fn main() {
    println!("Hello, world!");
}

fn ctv(_tx: &Transaction, input: usize) -> Vec<u8> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctv() {
        let tx = include_str!("../tests/tx.txt");
        let inputs = [0usize, 1, 3116999548, 4294967295];
        let results = [
            "2d28d0672f1d46cb3e86abd7e682d2d3e9961e6c9237157f47d39f0a694bb694",
            "12f7ab0a282fb9e29c9fd2ada21f950f492bfd5778a94202398c13ae6e97f0b4",
            "0ee9cc212182845d4c32ba6b3ba8859800d5cf423c58fb1444feaf21aa9cf81c",
            "da78ece7c0888725532355018961f58ad471f242e29a60adf84c55007fad608f",
        ];

        let tx = hex::decode(&tx).unwrap();
        let tx: Transaction = bitcoin::consensus::deserialize(&tx).unwrap();
        for input in inputs {
            let calculated = hex::encode(ctv(&tx, input));
            assert_eq!(calculated, results[input]);
        }
    }
}
