[![Latest version](https://img.shields.io/crates/v/cpp_map.svg)](https://crates.io/crates/cpp_map)
[![Documentation](https://docs.rs/cpp_map/badge.svg)](https://docs.rs/cpp_map)
[![Workflow](https://github.com/eadf/cpp_map.rs/workflows/Rust/badge.svg)](https://github.com/eadf/cpp_map.rs/workflows/Rust/badge.svg)
[![Workflow](https://github.com/eadf/cpp_map.rs/workflows/Clippy/badge.svg)](https://github.com/eadf/cpp_map.rs/workflows/Clippy/badge.svg)
[![dependency status](https://deps.rs/crate/cpp_map/0.1.1/status.svg)](https://deps.rs/crate/cpp_map/0.1.1)
![license](https://img.shields.io/crates/l/cpp_map)

# cpp_map.rs
A simple C++ std::map emulator for Rust. Probably not useful for anyone else.

I needed a data structure that could emulate a C++ std::map, particularly its pointer-based iterators.
More specifically, it needed to support the insertion position hint functionality, since the keys I’m using aren’t 
entirely transitive — i.e., whether you search for the insertion position from the head or the tail can make a big difference.

I also needed the ability to replace the key of an already-inserted item without altering the order.
(Don’t ask.)

Another quirk I had to replicate: in C++, std::map::insert(key, value) is a no-op if the key already exists — it won’t even use the new value.

The current implementation uses a doubly linked Vec list and only supports sequential search.

## License

Licensed under either of

* [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
* [MIT license](http://opensource.org/licenses/MIT)

at your option.
