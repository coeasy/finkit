// Preserve the generated/legacy C ABI surface and attach the hand-written
// research contract in a separate module so generators do not overwrite it.
include!("lib.rs");
mod research;
