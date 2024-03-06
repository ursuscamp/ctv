use regex::Regex;

pub fn colorize(script: &str) -> String {
    let opcode = Regex::new(r"(OP_\w+)").unwrap();
    let hex = Regex::new(r"([0-9a-z]{64})").unwrap();
    let color = opcode.replace_all(script, r#"<span style="color: red">$1</span>"#);
    let color = hex.replace_all(&color, r#"<span style="color: green">$1</span>"#);

    color.replace("OP_NOP4", "OP_CTV")
}
