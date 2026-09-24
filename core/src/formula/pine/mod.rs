//! Pine Script v5 subset parser and mapper.
//!
//! Parses a Pine Script v5 subset into `PineAst` and maps it to a formula
//! `AstNode`, which the engine then evaluates.
//!
//! There is deliberately **no** separate Pine execution engine. A bar-by-bar
//! Pine evaluator would be a second implementation of semantics the formula
//! backend already owns (na propagation, `nz`/`fixnan`, series history,
//! `barstate`), and the two would have to be kept numerically in step by hand —
//! the exact shape that produces silent divergence. Pine-specific concerns are
//! translated into engine primitives here in the mapper instead: `barstate.*`
//! becomes `BARPOS`/`BARSCOUNT` comparisons, and `request.security` goes through
//! the explicit [`PineSecurityResolver`] boundary.

pub mod ast_mapper;
pub mod builtin_table;
pub mod parser;

pub use ast_mapper::{
    map_pine_to_alphata, map_pine_to_alphata_with_security, PineMapperError, PineSecurityResolver,
};
pub use builtin_table::{BuiltinMapping, PineBuiltinTable};
pub use parser::{parse_pine, PineAst, PineAstNode, PineError, PineType};
