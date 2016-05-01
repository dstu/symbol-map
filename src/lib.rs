pub mod indexing;
mod table;  // Not pub because all pub symbols re-exported.

pub use self::table::{Symbol, SymbolId, Table, TableIter};
