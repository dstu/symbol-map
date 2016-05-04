//! Provides fast mapping of arbitrary values to whole-number identifiers.

pub mod indexing;
mod table;  // Not pub because all pub symbols re-exported.

#[cfg(test)] extern crate crossbeam;

pub use self::table::{Symbol, SymbolId, Table, TableIter};
