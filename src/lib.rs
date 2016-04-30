use std::default::Default;
use std::mem;
use std::sync::RwLock;

mod indexing;
mod symbol;

pub use self::indexing::{HashIndex, IndexingMethod, Insertion};
pub use self::symbol::{Symbol, SymbolId};

pub struct Table<T> {
    head: Option<Box<Symbol<T>>>,
    next_id: SymbolId,
}

impl<T> Table<T> {
    pub fn new() -> Self {
        Table {
            head: None,
            next_id: Default::default(),
        }
    }

    pub fn len(&self) -> usize {
        let SymbolId(len) = self.next_id;
        len
    }

    pub fn insert(&mut self, value: T) -> &Symbol<T> {
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

    pub fn iter<'s>(&'s self) -> Iter<'s, T> {
        Iter {
            remaining: self.len(),
            item: (&self.head).as_ref(),
        }
    }
}

pub struct Iter<'a, T> where T: 'a {
    remaining: usize,
    item: Option<&'a Box<Symbol<T>>>,
}

impl<'a, T> Iterator for Iter<'a, T> where T: 'a {
    type Item = &'a Symbol<T>;

    fn next(&mut self) -> Option<&'a Symbol<T>> {
        if self.item.is_none() {
            None
        } else {
            self.remaining -= 1;
            let item = self.item.unwrap();
            self.item = (&item.next).as_ref();
            Some(&item)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

pub struct Index<'a, M> where M: IndexingMethod<'a> {
    table: &'a mut Table<<M as IndexingMethod<'a>>::Data>,
    index: M,
}


impl<'a, M> Index<'a, M> where M: IndexingMethod<'a> {
    pub fn new(table: &'a mut Table<<M as IndexingMethod<'a>>::Data>) -> Self {
        let index = M::new(table);
        Index {
            table: table,
            index: index,
        }
    }

    pub fn get(&self, data: &<M as IndexingMethod<'a>>::Data)
               -> Option<&'a Symbol<<M as IndexingMethod<'a>>::Data>> {
        self.index.get(data)
    }

    pub fn get_or_insert(&self, data: <M as IndexingMethod<'a>>::Data)
                         -> Insertion<'a, <M as IndexingMethod<'a>>::Data> {
        self.index.get_or_insert(self.table, data)
    }
}
