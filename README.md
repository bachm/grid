# grid

A 2d array whose size is determined at runtime and is fixed at construction. Elements are stored in row-major order.

The array can be indexed with any type implementing the `Point2` trait defined in this library.
By default, `Point2` is implemented for `(u32, u32)` and `[u32; 2]`.

Serialization is supported via the `rustc_serialize` crate.

Various convenience functions are provided.