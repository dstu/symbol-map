use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use super::{Insertion, Symbol, Table};

/// Wrapper for a raw pointer which lets us treat it like a reference. No safety
/// checks or lifetimes protect this reference, so a `Ref<T>` may be invalidated
/// without warning.
///
/// Note that the impls for `Eq`, `Hash`, `Ord`, `PartialEq`, and `PartialOrd`
/// all dereference the raw pointer that this structure wraps. As a result, a
/// `Ref<T>` must be removed from any data structures that make use of any of
/// those interfaces *before* it is invalidated.
struct Ref<T> { ptr: *const T, }

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
pub trait IndexingMethod<'a> {
    /// The type `T` of a `Table<T>`.
    type Data: 'a;

    /// Returns a new indexing method that has already indexed the contents of
    /// `table`.
    fn from_table(table: &'a Table<Self::Data>) -> Self;

    /// Adds the contents of `table` to the index.
    fn index(&mut self, table: &'a Table<Self::Data>);

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

    /// Clears all indexed content.
    fn clear(&mut self);
}

/// HashMap-based indexing for a `Table` that has been borrowed for the lifetime
/// `'a`.
pub struct HashIndex<'a, T> where T: 'a + Eq + Hash {
    lifetime: PhantomData<&'a ()>,
    by_symbol: HashMap<Ref<T>, Ref<Symbol<T>>>,
    by_id: Vec<Ref<Symbol<T>>>,
}

impl<'a, T> IndexingMethod<'a> for HashIndex<'a, T> where T: 'a + Eq + Hash {
    type Data = T;

    fn from_table(table: &'a Table<T>) -> Self {
        let mut by_symbol = HashMap::with_capacity(table.len());
        let mut by_id = Vec::with_capacity(table.len());
        for symbol in table.iter() {
            by_symbol.insert(Ref::new(symbol.data()), Ref::new(symbol));
            by_id.push(Ref::new(symbol));
        }
        HashIndex {
            lifetime: PhantomData,
            by_symbol: by_symbol,
            by_id: by_id,
        }
    }

    fn index(&mut self, table: &'a Table<T>) {
        self.by_symbol.reserve(table.len());
        self.by_id.reserve(table.len());
        for symbol in table.iter() {
            self.by_symbol.insert(Ref::new(symbol.data()), Ref::new(symbol));
            self.by_id.push(Ref::new(symbol));
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

    fn clear(&mut self) {
        self.by_symbol.clear();
        self.by_id.clear();
    }
}
