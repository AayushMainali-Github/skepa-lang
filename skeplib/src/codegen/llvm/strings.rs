use crate::ir::{IrProgram, collect_program_string_constants};
use std::collections::HashMap;

pub fn collect_string_literals(program: &IrProgram) -> HashMap<String, String> {
    collect_program_string_constants(program)
        .into_iter()
        .enumerate()
        .map(|(index, value)| (value, format!("@.str.{index}")))
        .collect()
}

pub fn encode_c_string(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            32..=126 => out.push(byte as char),
            _ => out.push_str(&format!("\\{:02X}", byte)),
        }
    }
    out.push_str("\\00");
    out
}

pub fn runtime_string_symbol(raw_symbol: &str) -> String {
    raw_symbol.replacen("@.str.", "@.rtstr.", 1)
}
