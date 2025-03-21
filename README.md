# Seeded serialization

[![CI](https://github.com/timothee-haudebourg/serde-seeded/workflows/CI/badge.svg)](https://github.com/timothee-haudebourg/serde-seeded/actions)
[![Crate informations](https://img.shields.io/crates/v/serde-seeded.svg?style=flat-square)](https://crates.io/crates/serde-seeded)
[![License](https://img.shields.io/crates/l/serde-seeded.svg?style=flat-square)](https://github.com/timothee-haudebourg/serde-seeded#license)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/serde-seeded)

<!-- cargo-rdme start -->

This library provides types, traits and derive macros to deal with seeded
serialization/deserialization with serde.
- A `SerializeSeeded` trait and derive macro to serialize types with a seed.
- A `Seeded<Q, T>` type that implements `Serialize` calling
  `T::serialize_seeded` with a seed `Q`.
- A `DeserializeSeeded` trait and derive macro to deserialize types with a
  seed.
- A `Seed<Q, T>` type implementing `DeserializeSeed` calling
  `T::deserialize_seeded` with a seed `Q`.

See the `tests` folder to find some examples.

<!-- cargo-rdme end -->

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
