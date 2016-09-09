//! Provides fast mapping of arbitrary values to symbolic identifiers.
//!
//! The mapping to symbols is stored in the [Table](struct.Table.html) type,
//! which retains ownership of the values being mapped. Any type that implements
//! [SymbolId](trait.SymbolId.html) may be used as a symbol. Impls are provided
//! for Rust's default unsigned integer types.
//!
//! Fast bidirectional lookup on top of a Table is provided by the
//! [indexing](indexing/index.html) package, through the
//! [Indexing](indexing/trait.Indexing.html) trait. For convenience, a
//! HashMap-backed index is provided in
//! [HashIndexing](indexing/struct.HashIndexing.html).
//!
//! # Example
//!
//! ```
//! use symbol_map::indexing::{HashIndexing,Indexing};
//! use std::str::FromStr;
//!
//! let mut pos_index = HashIndexing::<String, usize>::default();
//! let s1 = String::from_str("NNP").unwrap();
//! let s2 = String::from_str("VBD").unwrap();
//! let s3 = String::from_str("NNP").unwrap();
//!
//! // We lose ownership of values passed to get_or_insert, so we pass in
//! // clones of our data.
//! {
//!     // The value returned by get_or_inset tells us whether a new association
//!     // was inserted, but we just unwrap it here. The resulting association
//!     // has a borrow of the index and its underlying symbol table, so we
//!     // restrict assoc to this inner scope in order to make additional
//!     // insertions below.
//!     let assoc = pos_index.get_or_insert(s1.clone()).unwrap();
//!     assert!(*assoc.id() == 0);
//!     assert!(assoc.data() == &s1);
//!     assert!(assoc.data() == &s3);
//! }
//! pos_index.get_or_insert(s2.clone());
//! pos_index.get_or_insert(s3.clone());
//! // Look up the values we just inserted.
//! let assoc1 = pos_index.get(&s1).unwrap();
//! let assoc2 = pos_index.get(&s2).unwrap();
//! let assoc3 = pos_index.get(&s3).unwrap();
//! assert!(assoc1.data() == &s1);
//! assert!(assoc1.data() == &s3);
//! assert!(*assoc1.id() == 0);
//! assert!(*assoc2.id() == 1);
//! assert!(*assoc3.id() == 0);
//! assert!(assoc1 != assoc2);
//! assert!(assoc1 == assoc3);
//! ```

pub mod indexing;
mod table;  // Not pub because all pub symbols re-exported.

#[cfg(test)] extern crate crossbeam;

pub use self::table::{Symbol, SymbolId, Table, TableIntoIter, TableIter};
