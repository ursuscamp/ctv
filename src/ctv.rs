use regex::Regex;

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
