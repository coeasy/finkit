//! Fuzz target for the formula parser and bytecode compiler.
//!
//! Contract: arbitrary UTF-8 input must never panic. Parse failures are
//! returned as `Err`; successful parses are compiled to bytecode without UB.
//!
//! # Running
//!
//! ```bash
//! cargo +nightly fuzz run fuzz_formula_parser -- -runs=50000
//! ```

#![no_main]

use alpha_ta_core::formula::bytecode::compile_to_bytecode;
use alpha_ta_core::formula::parser::parse_formula;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(src) = std::str::from_utf8(data) {
        if src.is_empty() || src.len() > 8192 {
            return;
        }

        if let Ok(ast) = parse_formula(src) {
            // Compilation must not panic on any successfully parsed AST.
            let _ = compile_to_bytecode(&ast, src);
        }
    }
});
