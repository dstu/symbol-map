use std::cmp::{Eq, Ord, Ordering, PartialEq};
use std::default::Default;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::Iterator;
use std::mem;

/// A table entry that associates an instance of `T` with an atomic symbol.
///
/// Types `T` should not be mutated by any means once they are associated with a
/// `SymbolId` and stored in a `Table`. Doing so may invalidate any caching or
/// indexing that is done on top of the table.
pub struct Symbol<T, D> where D: SymbolId {
    id: D,
    data: T,
    next: Option<Box<Symbol<T, D>>>,
}

impl<T, D> Symbol<T, D> where D: SymbolId {
    /// Returns the symbol's ID.
    pub fn id(&self) -> &D {
        &self.id
    }

    /// Returns a reference to the symbol's data.
    ///
    /// A `Symbol<T>` that is owned by a `Table` does not move in memory as long
    /// as it is not dropped from the table. As a result, you may retain a raw
    /// pointer to this data and dereference it as long as its parent
    /// `Symbol<T>` is not dropped.
    pub fn data(&self) -> &T {
        &self.data
    }
}

impl<T, D> Hash for Symbol<T, D> where T: Hash, D: SymbolId {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.data.hash(state)
    }
}

impl<T, D> PartialEq for Symbol<T, D> where T: PartialEq, D: SymbolId {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
    }
}

impl<T, D> Eq for Symbol<T, D> where T: Eq, D: SymbolId { }

impl<T, D> PartialOrd for Symbol<T, D> where T: PartialOrd, D: SymbolId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl<T, D> Ord for Symbol<T, D> where T: Ord, D: SymbolId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.data.cmp(&other.data)
    }
}

/// An atomic ID.
pub trait SymbolId:
Copy + Clone + fmt::Debug + Default + Eq + Hash + Ord + PartialEq + PartialOrd + Send + Sync {
    /// Returns the ID immediately subsequent to this one.
    fn next(&self) -> Self;

    /// Casts the ID to a `usize`.
    fn as_usize(&self) -> usize;
}

impl SymbolId for usize {
    fn next(&self) -> Self { *self + 1 }
    fn as_usize(&self) -> usize { *self }
}

impl SymbolId for u8 {
    fn next(&self) -> Self { *self + 1 }

    fn as_usize(&self) -> usize { *self as usize }
}

impl SymbolId for u16 {
    fn next(&self) -> Self { *self + 1 }
    fn as_usize(&self) -> usize { *self as usize }
}

impl SymbolId for u32 {
    fn next(&self) -> Self { *self + 1 }
    fn as_usize(&self) -> usize { *self as usize }
}

impl SymbolId for u64 {
    fn next(&self) -> Self { *self + 1 }
    fn as_usize(&self) -> usize { *self as usize }
}

/// The head of a linked list associating `T`s with `SymbolId`s. `SymbolId`
/// values start at 0 and increase by 1 for each `T` added to the table.
///
/// The linked list owns instances of `Symbol<T>`, which wrap around a `T` and a
/// `SymbolId`. It satisfies the contract: *once allocated, a Symbol<T>'s
/// address does not change as long as its parent table exists and it is not
/// dropped from the table*.
///
/// As a result, a table index may retain a raw pointer to a `Symbol<T>` as long
/// as care is taken not to dereference or otherwise make use of such pointers
/// after the symbol they point to has been dropped by `retain()`.
pub struct Table<T, D> where D: SymbolId {
    head: Option<Box<Symbol<T, D>>>,
    next_id: D,
}

impl<T, D> Table<T, D> where D: SymbolId {
    /// Creates a new, empty table.
    pub fn new() -> Self {
        Table {
            head: None,
            next_id: Default::default(),
        }
    }

    /// Returns the number of symbols in the table.
    pub fn len(&self) -> usize {
        self.next_id.as_usize()
    }

    /// Inserts `value` into the table and assigns it an id. The same value may
    /// be inserted more than once. To prevent such operations, use the
    /// `get_or_insert()` method of `Index`.
    ///
    /// Returns a reference to the newly created symbol.
    pub fn insert(&mut self, value: T) -> &Symbol<T, D> {
        let next_id = self.next_id;
        self.next_id = self.next_id.next();
        let mut new_head = Box::new(Symbol {
            id: next_id,
            data: value,
            next: None,
        });
        mem::swap(&mut self.head, &mut new_head.next);
        self.head = Some(new_head);
        (&self.head).as_ref().unwrap()
    }

    /// Returns an iterator over table entries.
    pub fn iter<'s>(&'s self) -> TableIter<'s, T, D> {
        TableIter {
            remaining: self.len(),
            item: (&self.head).as_ref(),
        }
    }

    /// Sets `value` as the head of this list, assigning it a new `SymbolId` as
    /// if it were added by `insert()`. If `value` is already the head of
    /// another list, its subsequent list elements are dropped.
    fn emplace_head(&mut self, mut value: Box<Symbol<T, D>>) {
        let next_id = self.next_id;
        self.next_id = self.next_id.next();
        value.id = next_id;
        mem::swap(&mut value.next, &mut self.head);
        mem::swap(&mut self.head, &mut Some(value));
    }

    /// Drops all table entries which do not satisfy `predicate`. The address of
    /// `Symbol<T>`s for entries which are retained does not change. The
    /// `SymbolId`s associated with table entries may change arbitrarily (but
    /// will remain a dense range of unique values starting at 0).
    pub fn retain<F>(&mut self, mut predicate: F)  where F: FnMut(&Symbol<T, D>) -> bool {
        // Destructively walk linked list, removing elements for which
        // predicate(symbol) returns false, reassigning `SymbolId`s as we
        // go. This is done in place, without making new allocations for the
        // elements that we retain.
        let mut retained = Table::new();
        let mut head = None;
        mem::swap(&mut head, &mut self.head);
        loop {
            head = match head {
                None => break,
                Some(mut symbol) =>
                    if predicate(&symbol) {
                        let mut next_head = None;
                        mem::swap(&mut next_head, &mut symbol.next);
                        retained.emplace_head(symbol);
                        next_head
                    } else {
                        symbol.next
                    },
            }
        }
        mem::swap(self, &mut retained);
    }
}

/// Iterator over table contents.
pub struct TableIter<'a, T, D> where T: 'a, D: 'a + SymbolId {
    remaining: usize,
    item: Option<&'a Box<Symbol<T, D>>>,
}

impl<'a, T, D> Iterator for TableIter<'a, T, D> where T: 'a, D: 'a + SymbolId {
    type Item = &'a Symbol<T, D>;

    fn next(&mut self) -> Option<&'a Symbol<T, D>> {
        let mut item = None;
        mem::swap(&mut item, &mut self.item);
        match item {
            None => None,
            Some(symbol) => {
                self.remaining -= 1;
                self.item = symbol.next.as_ref();
                Some(symbol)
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

#[cfg(test)]
mod test {
    use super::{Symbol, SymbolId, Table};

    use std::default::Default;

    const VALUES: &'static [usize] = &[101, 203, 500, 30, 0, 1];

    #[test]
    fn symbol_id_ok() {
        let id: usize = Default::default();
        assert_eq!(id.as_usize(), 0);
        assert_eq!(id.next().as_usize(), 1);
        assert_eq!(id.next().next().as_usize(), 2);
        assert_eq!(id.as_usize(), 0);
    }

    #[test]
    fn new_table_empty_ok() {
        let t = Table::<usize, usize>::new();
        assert!(t.head.is_none());
        assert!(t.next_id == 0);
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn table_insert_ok() {
        let mut t = Table::<usize, usize>::new();
        for (i, v) in VALUES.iter().enumerate() {
            t.insert(*v);
            assert_eq!(t.len(), i + 1);
            assert_eq!(t.next_id.as_usize(), i + 1);
            assert_eq!(t.head.as_ref().map(|x| x.data), Some(*v));
        }
        assert_eq!(t.len(), VALUES.len());
        assert_eq!(t.next_id.as_usize(), VALUES.len());

        let mut x = t.head.as_ref();
        let mut count = 0;
        let mut vs = VALUES.iter().rev().enumerate();
        loop {
            x = match x {
                None => break,
                Some(symbol) => {
                    let (i, v) = vs.next().unwrap();
                    assert_eq!(i, count);
                    assert_eq!(symbol.data(), v);
                    count += 1;
                    symbol.next.as_ref()
                },
            }
        }
        assert_eq!(vs.next(), None);
    }

    #[test]
    fn table_empty_iter_ok() {
        let t = Table::<usize, usize>::new();
        let mut i = t.iter();
        assert_eq!(i.size_hint(), (0, Some(0)));
        assert!(i.next().is_none());
        assert_eq!(i.size_hint(), (0, Some(0)));
    }

    #[test]
    fn table_iter_ok() {
        let mut t = Table::<usize, u32>::new();
        for v in VALUES.iter() {
            t.insert(*v);
        }
        assert_eq!(t.len(), VALUES.len());

        let mut i = t.iter();
        let mut expected_len = t.len();
        let mut vs = VALUES.iter().rev();
        assert_eq!(i.size_hint(), (expected_len, Some(expected_len)));
        while let Some(symbol) = i.next() {
            expected_len -= 1;
            assert_eq!(i.size_hint(), (expected_len, Some(expected_len)));
            assert_eq!(Some(symbol.data()), vs.next());
        }
        assert_eq!(i.size_hint(), (0, Some(0)));
    }

    #[test]
    fn moved_table_internal_address_unchanged_ok() {
        let mut stack_table = Table::<usize, u8>::new();
        let mut original_data_addresses = Vec::new();
        let mut original_symbol_addresses = Vec::new();
        for v in VALUES.iter() {
            let symbol = stack_table.insert(*v);
            assert_eq!(*symbol.data(), *v);
            original_data_addresses.push(symbol.data() as *const usize);
            original_symbol_addresses.push(symbol as *const Symbol<usize, u8>);
        }

        let heap_table = Box::new(stack_table);
        let mut count =0;
        for (symbol, (value, (data_address, symbol_address))) in heap_table.iter().zip(
            VALUES.iter().rev().zip(
                original_data_addresses.into_iter().rev().zip(
                    original_symbol_addresses.into_iter().rev()))) {
            assert_eq!(symbol.data(), value);
            assert_eq!(symbol.data() as *const usize, data_address);
            assert_eq!(symbol as *const Symbol<usize, u8>, symbol_address);
            count += 1;
        }
        assert_eq!(count, VALUES.len());
    }
}
