use std::mem;

mod indexing;
mod table;

pub use self::indexing::{HashIndex, IndexingMethod};
pub use self::table::{Symbol, SymbolId, Table, TableIter};

/// Indicates whether a symbol lookup had to create a new table entry.
pub enum Insertion<'a, T> where T: 'a {
    /// Symbol was already present in table.
    Present(&'a Symbol<T>),
    /// Symbol was not present in table, and a new entry was created for it.
    New(&'a Symbol<T>),
}

/// Provides indexing for a `Table`, so that its elements may be retrieved
/// efficiently. Most table lookups should go through this structure instead of
/// a `Table` directly.
///
/// An `Indexed<'a, M>` borrows an underlying `Table<M::Data>` for the lifetime
/// `'a`. This table provides persistent storage for `Symbol<M::Data>`s, which
/// associate instances of `M::Data` with a `SymbolId`.
pub struct Indexed<'a, M> where M: IndexingMethod<'a> {
    table: &'a mut Table<<M as IndexingMethod<'a>>::Data>,
    index: M,
}

unsafe fn cast_table<'s, 'a, T>(reference: &'s mut Table<T>) -> &'a mut Table<T> {
    mem::transmute(reference)
}

impl<'a, M> Indexed<'a, M> where M: IndexingMethod<'a> {
    /// Creates a new index wrapping around `table`.
    pub fn new(table: &'a mut Table<<M as IndexingMethod<'a>>::Data>) -> Self {
        let index = unsafe {
            let table = table as *mut Table<<M as IndexingMethod<'a>>::Data>;
            M::from_table(&mut *table)
        };
        Indexed {
            table: table,
            index: index,
        }
    }

    /// Delegates to `M::get()`.
    pub fn get(&self, data: &<M as IndexingMethod<'a>>::Data)
               -> Option<&'a Symbol<<M as IndexingMethod<'a>>::Data>> {
        unsafe { self.index.get(data) }
    }

    /// Delegates to `M::get_or_insert()`.
    pub fn get_or_insert<'s>(&'s mut self, data: <M as IndexingMethod<'a>>::Data)
                             -> Insertion<'a, <M as IndexingMethod<'a>>::Data> {
        unsafe {
            self.index.get_or_insert(cast_table(self.table), data)
        }
    }

    pub fn retain<'s, F>(&'s mut self, predicate: F)
        where F: FnMut(&Symbol<<M as IndexingMethod<'a>>::Data>) -> bool {
        self.index.clear();
        self.table.retain(predicate);
        self.index.index(unsafe { cast_table(self.table) });
    }
}
