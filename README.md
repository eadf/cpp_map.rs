[![Crates.io](https://meritbadge.herokuapp.com/cpp_map)](https://crates.io/crates/cpp_map)
[![Documentation](https://docs.rs/cpp_map/badge.svg)](https://docs.rs/cpp_map)
[![Workflow](https://github.com/eadf/cpp_map.rs/workflows/Rust/badge.svg)](https://github.com/eadf/cpp_map.rs/workflows/Rust/badge.svg)
[![Workflow](https://github.com/eadf/cpp_map.rs/workflows/Clippy/badge.svg)](https://github.com/eadf/cpp_map.rs/workflows/Clippy/badge.svg)
[![dependency status](https://deps.rs/crate/cpp_map/0.0.1/status.svg)](https://deps.rs/crate/cpp_map/0.0.1)


# cpp_map.rs
A simple C++ map emulator for Rust. Probably not useful for anyone. 

I needed a data structure that could emulate a C++ map, and it's pointer based interators.
More specifically it needs to emulate the insertion position hint functionality as the keys I intend to 
use are not entirely transitive. i.e., searching for insertion position from the head or tail makes a big difference.

The current implementation uses a double linked Vec list and it only supports linear search.

## License

Licensed under either of

* [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
* [MIT license](http://opensource.org/licenses/MIT)

at your option.
