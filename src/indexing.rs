use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;

use super::Symbol;
use super::Table;

struct Ref<T> { ptr: *const T, }

impl<T> Ref<T> {
    fn new(data: &T) -> Self {
        Ref { ptr: data as *const T, }
    }

    fn deref<'a>(&self) -> &'a T {
        unsafe { &*self.ptr }
    }
}

impl<T> Hash for Ref<T> where T: Hash {
    fn hash<H>(&self, h: &mut H) where H: Hasher {
        unsafe { (*self.ptr).hash(h) }
    }
}

impl<T> PartialEq for Ref<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        unsafe { (*self.ptr).eq(&(*other.ptr)) }
    }
}

impl<T> Eq for Ref<T> where T: Eq { }

pub enum Insertion<'a, T> where T: 'a {
    Present(&'a Symbol<T>),
    New(&'a Symbol<T>),
}

pub trait IndexingMethod<'a> {
    type Data: 'a;

    fn new(table: &'a Table<Self::Data>) -> Self;

    fn get(&self, data: &Self::Data) -> Option<&'a Symbol<Self::Data>>;

    fn get_or_insert(&mut self, table: &'a mut Table<Self::Data>, data: Self::Data)
                     -> Insertion<'a, Self::Data>;
}

pub struct HashIndex<'a, T> where T: 'a + Eq + Hash {
    lifetime: PhantomData<&'a ()>,
    by_symbol: HashMap<Ref<T>, Ref<Symbol<T>>>,
    by_id: Vec<Ref<Symbol<T>>>,
}

impl<'a, T> IndexingMethod<'a> for HashIndex<'a, T> where T: 'a + Eq + Hash {
    type Data = T;

    fn new(table: &'a Table<T>) -> Self {
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

    fn get(&self, data: &T) -> Option<&'a Symbol<T>> {
        self.by_symbol.get(&Ref::new(data)).map(|x| x.deref())
    }

    fn get_or_insert(&mut self, table: &'a mut Table<T>, data: T) -> Insertion<'a, T> {
        use std::collections::hash_map::Entry;
        if let Entry::Occupied(e) = self.by_symbol.entry(unsafe { mem::transmute(&data) }) {
            return Insertion::Present(e.get().deref())
        }
        // TODO: when the HashMap API gets revised, we may be able to do this
        // without a second hashtable lookup.
        let symbol = table.insert(data);
        self.by_symbol.insert(Ref::new(symbol.data()), Ref::new(symbol));
        self.by_id.push(Ref::new(symbol));
        Insertion::New(symbol)
    }
}
