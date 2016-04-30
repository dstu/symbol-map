use std::default::Default;
use std::cmp::{Eq, Ord, Ordering, PartialEq};
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SymbolId(pub usize);

impl SymbolId {
    pub fn next(&self) -> Self {
        let SymbolId(x) = *self;
        SymbolId(x + 1)
    }
}

impl Default for SymbolId {
    fn default() -> Self {
        SymbolId(0)
    }
}

pub struct Symbol<T> {
    pub id: SymbolId,
    pub data: T,
    pub next: Option<Box<Symbol<T>>>,
}

impl<T> Symbol<T> {
    pub fn id(&self) -> SymbolId {
        self.id
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}

impl<T> Hash for Symbol<T> where T: Hash {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.data.hash(state)
    }
}

impl<T> PartialEq for Symbol<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
    }
}

impl<T> Eq for Symbol<T> where T: Eq { }

impl<T> PartialOrd for Symbol<T> where T: PartialOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl<T> Ord for Symbol<T> where T: Ord {
    fn cmp(&self, other: &Self) -> Ordering {
        self.data.cmp(&other.data)
    }
}

