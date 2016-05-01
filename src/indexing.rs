use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::mem;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use super::{Insertion, Symbol, SymbolId, Table};

/// Wrapper for a raw pointer which lets us treat it like a reference. No safety
/// checks or lifetimes protect this reference, so a `Ref<T>` may be invalidated
/// without warning.
///
/// Note that the impls for `Debug`, `Eq`, `Hash`, `Ord`, `PartialEq`, and
/// `PartialOrd` all dereference the raw pointer that this structure wraps. As a
/// result, a `Ref<T>` must be removed from any data structures that make use of
/// any of those interfaces *before* it is invalidated.
pub struct Ref<T> { ptr: *const T, }

impl<T> Ref<T> {
    /// Casts `data` to `*const T` and retains the pointer for dereferencing at
    /// some point in the future.
    fn new(data: &T) -> Self {
        Ref { ptr: data as *const T, }
    }

    /// Dereferences the wrapped pointer. The explicit lifetime parameter should
    /// match the lifetime of the parent `Table` that the wrapped pointer points
    /// into. Care should be taken not to call this method if the integrity of
    /// the reference passed to `new()` cannot be verified.
    unsafe fn deref<'a>(&self) -> &'a T {
        &*self.ptr
    }
}

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Ref { ptr: self.ptr, }
    }
}

impl<T> Copy for Ref<T> { }

impl<T> fmt::Pointer for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}

impl<T> fmt::Debug for Ref<T> where T: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ref({:?})", unsafe { &(*self.ptr) })
    }
}

impl<T> Eq for Ref<T> where T: Eq { }

impl<T> Hash for Ref<T> where T: Hash {
    fn hash<H>(&self, h: &mut H) where H: Hasher {
        unsafe { (*self.ptr).hash(h) }
    }
}

impl<T> Ord for Ref<T> where T: Ord {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe { (*self.ptr).cmp(&(*other.ptr)) }
    }
}

impl<T> PartialEq for Ref<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        unsafe { (*self.ptr).eq(&(*other.ptr)) }
    }
}

impl<T> PartialOrd for Ref<T> where T: PartialOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unsafe { (*self.ptr).partial_cmp(&(*other.ptr)) }
    }
}

/// Interface for indexing a `Table`.
///
/// This trait is publicly exposed for extensibility. Realistically speaking,
/// however, you should probably just use `HashIndexing`.
pub trait IndexingMethod<'a>: Sized {
    /// The type `T` of a `Table<T>`.
    type Data: 'a;

    /// Returns a new indexing method that has already indexed the contents of
    /// `table`.
    fn from_table(table: &'a Table<Self::Data>) -> Self;

    /// Overwrites current index with a new index for `table`.
    fn index(&mut self, table: &'a Table<Self::Data>) {
        self.clear();
        let mut new_table = Self::from_table(table);
        mem::swap(self, &mut new_table);
    }

    /// Looks up `data` in the index. Returns `Some(symbol)` if a symbol is
    /// present, else `None`.
    ///
    /// This method is unsafe because it may choose to dereference a raw pointer
    /// into a `Table<Self::Data>`. Callers should ensure that any such
    /// references are valid.
    unsafe fn get(&self, data: &Self::Data) -> Option<&'a Symbol<Self::Data>>;

    /// Looks up `data` in the index, inserting it into the index and `table` if
    /// it isn't present. Returns the resulting `Symbol<T>` wrapped in an
    /// `Insertion` that indicates whether a new table entry had to be created.
    ///
    /// This method is unsafe because it may choose to dereference a raw pointer
    /// into a `Table<Self::Data>`. Callers should ensure that any such
    /// references are valid.
    unsafe fn get_or_insert(&mut self, table: &'a mut Table<Self::Data>, data: Self::Data)
                            -> Insertion<'a, Self::Data>;

    /// Looks up the symbol with id `i` in the index. Returns `Some(symbol)` if
    /// a symbol is present, else `None`.
    ///
    /// This method is unsafe because it may choose to dereference a raw pointer
    /// into a `Table<Self::Data>`. Callers should ensure that any such
    /// references are valid.
    unsafe fn get_symbol<'s>(&'s self, id: &SymbolId) -> Option<&'a Symbol<Self::Data>>;

    /// Clears all indexed content.
    fn clear(&mut self);
}

/// HashMap-based indexing for a `Table` that has been borrowed for the lifetime
/// `'a`.
pub struct HashIndexing<'a, T> where T: 'a + Eq + Hash {
    lifetime: PhantomData<&'a ()>,
    by_symbol: HashMap<Ref<T>, Ref<Symbol<T>>>,
    by_id: Vec<Ref<Symbol<T>>>,
}

impl<'a, T> IndexingMethod<'a> for HashIndexing<'a, T> where T: 'a + Eq + Hash {
    type Data = T;

    fn from_table(table: &'a Table<T>) -> Self {
        let mut by_symbol = HashMap::with_capacity(table.len());
        let mut by_id =
            match table.iter().next() {
                Some(symbol) => vec![Ref::new(symbol); table.len()],
                None => Vec::new(),
            };
        for symbol in table.iter() {
            by_symbol.insert(Ref::new(symbol.data()), Ref::new(symbol));
            by_id[symbol.id().as_usize()] = Ref::new(symbol);
        }
        HashIndexing {
            lifetime: PhantomData,
            by_symbol: by_symbol,
            by_id: by_id,
        }
    }

    unsafe fn get(&self, data: &T) -> Option<&'a Symbol<T>> {
        self.by_symbol.get(&Ref::new(data)).map(|x| x.deref())
    }

    unsafe fn get_or_insert(&mut self, table: &'a mut Table<T>, data: T) -> Insertion<'a, T> {
        use std::collections::hash_map::Entry;
        if let Entry::Occupied(e) = self.by_symbol.entry(Ref::new(&data)) {
            // Unsafe call to Ref::deref(): should be fine as long as caller
            // respects integrity of underlying table.
            return Insertion::Present(e.get().deref())
        }
        // TODO: when the HashMap API gets revised, we may be able to do this
        // without a second hashtable lookup.
        let symbol = table.insert(data);
        // The Ref that gets inserted has to be backed by data in the table, not
        // data on the stack (which is how we did the previous lookup).
        self.by_symbol.insert(Ref::new(symbol.data()), Ref::new(symbol));
        self.by_id.push(Ref::new(symbol));
        Insertion::New(symbol)
    }

    unsafe fn get_symbol(&self, id: &SymbolId) -> Option<&'a Symbol<T>> {
        self.by_id.get(id.as_usize()).map(|x| x.deref())
    }

    fn clear(&mut self) {
        self.by_symbol.clear();
        self.by_id.clear();
    }
}

/// BTreeMap-based indexing for a `Table` that has been borrowed for the
/// lifetime `'a`.
pub struct BTreeIndexing<'a, T> where T: 'a + Eq + Ord {
    lifetime: PhantomData<&'a ()>,
    by_symbol: BTreeMap<Ref<T>, Ref<Symbol<T>>>,
    by_id: Vec<Ref<Symbol<T>>>,
}

impl<'a, T> IndexingMethod<'a> for BTreeIndexing<'a, T> where T: 'a + Eq + Ord {
    type Data = T;

    fn from_table(table: &'a Table<T>) -> Self {
        let mut by_symbol = BTreeMap::new();
        let mut by_id =
            match table.iter().next() {
                Some(symbol) => vec![Ref::new(symbol); table.len()],
                None => Vec::new(),
            };
        for symbol in table.iter() {
            by_symbol.insert(Ref::new(symbol.data()), Ref::new(symbol));
            by_id[symbol.id().as_usize()] = Ref::new(symbol);
        }
        BTreeIndexing {
            lifetime: PhantomData,
            by_symbol: by_symbol,
            by_id: by_id,
        }
    }

    unsafe fn get(&self, data: &T) -> Option<&'a Symbol<T>> {
        self.by_symbol.get(&Ref::new(data)).map(|x| x.deref())
    }

    unsafe fn get_or_insert(&mut self, table: &'a mut Table<T>, data: T) -> Insertion<'a, T> {
        use std::collections::btree_map::Entry;
        if let Entry::Occupied(e) = self.by_symbol.entry(Ref::new(&data)) {
            // Unsafe call to Ref::deref(): should be fine as long as caller
            // respects integrity of underlying table.
            return Insertion::Present(e.get().deref())
        }
        // TODO: when the BTreeMap API gets revised, we may be able to do this
        // without a second hashtable lookup.
        let symbol = table.insert(data);
        // The Ref that gets inserted has to be backed by data in the table, not
        // data on the stack (which is how we did the previous lookup).
        self.by_symbol.insert(Ref::new(symbol.data()), Ref::new(symbol));
        self.by_id.push(Ref::new(symbol));
        Insertion::New(symbol)
    }

    unsafe fn get_symbol(&self, id: &SymbolId) -> Option<&'a Symbol<T>> {
        self.by_id.get(id.as_usize()).map(|x| x.deref())
    }

    fn clear(&mut self) {
        self.by_symbol.clear();
        self.by_id.clear();
    }
}

#[cfg(test)]
mod test {
    use super::{HashIndexing, IndexingMethod, Ref};
    use ::Table;

    use std::cmp::Ordering;
    use std::hash::{Hash, Hasher, SipHasher};
    use std::str::FromStr;

    const VALUES: &'static [usize] = &[101, 203, 500, 30, 0, 1];

    #[test]
    fn ref_impls_ok() {
        let x1 = String::from_str("foo").unwrap();
        let x2 = String::from_str("foo").unwrap();
        let x3 = String::from_str("fo").unwrap();
        let x4 = String::from_str("fox").unwrap();
        assert!(x1 == x2);
        assert!(&x1 as *const String != &x2 as *const String);

        let r1 = Ref::new(&x1);
        let r2 = Ref::new(&x2);
        let r3 = Ref::new(&x3);
        let r4 = Ref::new(&x4);
        // Eq, PartialEq.
        assert_eq!(r1, r2);
        assert!(r1 != r3);
        assert!(r1 != r4);

        // Ord (skip PartialOrd).
        assert_eq!(r1.cmp(&r2), Ordering::Equal);
        assert_eq!(r1.cmp(&r3), Ordering::Greater);
        assert_eq!(r1.cmp(&r4), Ordering::Less);

        // Hash.
        let mut rh1 = SipHasher::new();
        let mut rh2 = SipHasher::new();
        let mut rh3 = SipHasher::new();
        let mut rh4 = SipHasher::new();
        r1.hash(&mut rh1);
        r2.hash(&mut rh2);
        r3.hash(&mut rh3);
        r4.hash(&mut rh4);
        let rh1 = rh1.finish();
        let rh2 = rh2.finish();
        let rh3 = rh3.finish();
        let rh4 = rh4.finish();

        let mut xh1 = SipHasher::new();
        let mut xh2 = SipHasher::new();
        let mut xh3 = SipHasher::new();
        let mut xh4 = SipHasher::new();
        x1.hash(&mut xh1);
        x2.hash(&mut xh2);
        x3.hash(&mut xh3);
        x4.hash(&mut xh4);
        let xh1 = xh1.finish();
        let xh2 = xh2.finish();
        let xh3 = xh3.finish();
        let xh4 = xh4.finish();

        assert_eq!(rh1, xh1);
        assert_eq!(rh2, xh2);
        assert_eq!(rh3, xh3);
        assert_eq!(rh4, xh4);
    }

    #[test]
    fn hash_index_empty_ok() {
        let t = Table::<usize>::new();
        assert_eq!(t.len(), 0);
        let i = HashIndexing::from_table(&t);
        assert!(i.by_symbol.is_empty());
        assert!(i.by_id.is_empty());
    }

    #[test]
    fn hash_index_from_table_ok() {
        let mut t = Table::<usize>::new();
        for v in VALUES.iter() {
            t.insert(*v);
        }

        let i = HashIndexing::from_table(&t);
        assert_eq!(i.by_symbol.len(), t.len());
        assert_eq!(i.by_id.len(), t.len());
        for symbol in t.iter() {
            let data_ref = Ref::new(symbol.data());
            unsafe {
                assert_eq!(i.by_symbol.get(&data_ref).unwrap().deref().data(), symbol.data());
                assert_eq!(i.by_symbol.get(&data_ref).unwrap().deref().id(), symbol.id());
                assert_eq!(i.by_id[symbol.id().as_usize()].deref().data(), symbol.data());
            }
        }
    }
}
