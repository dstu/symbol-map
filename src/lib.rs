use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;

pub mod indexing;
mod table;  // Not pub because all pub symbols re-exported.

use self::indexing::{HashIndexing, IndexingMethod};
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
    pub fn get_or_insert(&mut self, data: <M as IndexingMethod<'a>>::Data)
                         -> Insertion<'a, <M as IndexingMethod<'a>>::Data> {
        unsafe {
            self.index.get_or_insert(cast_table(self.table), data)
        }
    }

    /// Delegates to `M::get_symbol()`.
    pub fn get_symbol(&self, id: &SymbolId) -> Option<&'a Symbol<<M as IndexingMethod<'a>>::Data>> {
        unsafe {
            self.index.get_symbol(id)
        }
    }

    /// Delegates to `Table::retain()`, safely clearing and rebuilding the
    /// index.
    pub fn retain<'s, F>(&'s mut self, predicate: F)
        where F: FnMut(&Symbol<<M as IndexingMethod<'a>>::Data>) -> bool {
        self.index.clear();
        self.table.retain(predicate);
        self.index.index(unsafe { cast_table(self.table) });
    }

    pub fn to_ro<'b>(&'b self) -> RoIndexed<'a, 'b, M> {
        RoIndexed {
            lifetime: PhantomData,
            index: &self.index,
        }
    }
}

pub struct RoIndexed<'a, 'b, M> where 'a: 'b, M: 'b + IndexingMethod<'a> {
    lifetime: PhantomData<&'a ()>,
    index: &'b M,
}

impl<'a, 'b, M> RoIndexed<'a, 'b, M> where 'b: 'a, M: IndexingMethod<'a> {
    pub fn get(&self, data: &<M as IndexingMethod<'a>>::Data)
               -> Option<&'a Symbol<<M as IndexingMethod<'a>>::Data>> {
        unsafe { self.index.get(data) }
    }

    pub fn get_symbol(&self, id: &SymbolId) -> Option<&'a Symbol<<M as IndexingMethod<'a>>::Data>> {
        unsafe { self.index.get_symbol(id) }
    }
}

/// Returns a new hashtable-based index for symbols in `table`.
pub fn hash_index<'a, T>(table: &'a mut Table<T>) -> Indexed<'a, HashIndexing<'a, T>>
    where T: 'a + Eq + Hash {
    Indexed::new(table)
}

#[cfg(test)]
mod test {
    use super::{Insertion, Table, hash_index};
    use super::indexing::IndexingMethod;

    const VALUES: &'static [usize] = &[101, 203, 500, 30, 0, 1];

    #[test]
    fn indexed_empty_ok() {
        let mut t = Table::<usize>::new();
        let _ = hash_index(&mut t);
    }

    #[test]
    fn indexed_empty_insertion_ok() {
        let mut t = Table::<usize>::new();
        let mut i = hash_index(&mut t);

        for v in VALUES.iter() {
            assert!(i.get(v).is_none());
            let id = match i.get_or_insert(*v) {
                Insertion::Present(_) => panic!(),
                Insertion::New(symbol) => {
                    assert_eq!(symbol.data(), v);
                    symbol.id()
                },
            };
            assert_eq!(i.get_symbol(&id).unwrap().data(), v);
        }
    }

    #[test]
    fn indexed_present_ok() {
        let mut t = Table::<usize>::new();
        for v in VALUES.iter() {
            t.insert(*v);
        }

        let mut i = hash_index(&mut t);
        for v in VALUES.iter() {
            assert_eq!(i.get(v).unwrap().data(), v);
            let id = match i.get_or_insert(*v) {
                Insertion::New(_) => panic!(),
                Insertion::Present(symbol) => {
                    assert_eq!(symbol.data(), v);
                    symbol.id()
                },
            };
            assert_eq!(i.get_symbol(&id).unwrap().data(), v);
        }
    }

    #[test]
    fn send_to_thread_ok() {
        use std::thread;

        let mut t = Table::<usize>::new();
        for v in VALUES.iter() {
            t.insert(*v);
        }
        let mut i = hash_index(&mut t);
        {
            let v1 = i.get(&VALUES[0]).unwrap();
            let index = i.to_ro();

            let t1 = thread::spawn(move || index.get_symbol(&v1.id()));
            match t1.join() {
                Ok(x) => {
                    assert_eq!(x.unwrap().id(), v1.id());
                    assert_eq!(x.unwrap().data(), v1.data());
                },
                _ => panic!(),
            }
        }
    }
}
