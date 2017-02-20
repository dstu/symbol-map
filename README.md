# Overview

This crate provides the means for associating values of an arbitrary type `T`
with values of another arbitrary type `S` ("symbols"). Typical use cases involve
values of `T` that are less convenient to deal with than values of `S`, in which
the identity of a `T` is important but its value is not. For example, the first
step in many natural language handling pipelines is construct a mapping from
token strings to a dense range of integers starting at 0, so that they may be
compared very quickly and used to index simple data structures like vectors and
arrays.

You can get a lot of this package's functionality with a structure like:

```rust
pub struct Table<T> where T: Hash + Eq {
    // Mapping from T to usize.
    by_symbol: HashMap<T, usize>,
    // Mapping from usize to T.
    by_id: Vec<T>,
}
```

But then you're maintaining two copies of each `T`. You can resort to a
`HashMap<Rc<T>, usize>` and `Vec<Rc<T>>`, but that forces you to pay the cost of
reference counting and isn't easily shared across threads. Using `Arc` instead
of `Rc` makes your type `Send`-able, but that has even more overhead than using
`Rc`.

Or you could use `symbol_table::HashIndexing<Data=T, SymbolId=usize>` and get a
type that is `Send` and `Sync` when `T` is and owns only one `T` per association
in the table.

See the [rustdoc](http://dstu.github.io/symbol-map/index.html) for example usage
and further technical details.

# Copyright

Copyright 2016, Donald S. Black.

Licensed under the Apache License, Version 2.0 (the “License”); you may not use
this file except in compliance with the License. You may obtain a copy of the
License at http://www.apache.org/licenses/LICENSE-2.0.

Unless required by applicable law or agreed to in writing, software distributed
under the License is distributed on an “AS IS” BASIS, WITHOUT WARRANTIES OR
CONDITIONS OF ANY KIND, either express or implied. See the License for the
specific language governing permissions and limitations under the License.
