//! Pine Script v5 subset parser and runtime.
//!
//! Parses a Pine Script v5 subset into `PineAst`, maps to AlphaTA `AstNode`,
//! and provides bar-by-bar series evaluation semantics.

pub mod ast_mapper;
pub mod builtin_table;
pub mod parser;
pub mod runtime;

pub use ast_mapper::{map_pine_to_alphata, PineMapperError};
pub use builtin_table::{BuiltinMapping, PineBuiltinTable};
pub use parser::{parse_pine, PineAst, PineAstNode, PineError, PineType};
pub use runtime::{PineRuntime, PineRuntimeError, SeriesValue};
