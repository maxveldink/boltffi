//! Encoded values between Ruby objects and BoltFFI wire bytes.
//!
//! `read` renders Rust-to-Ruby decoding and `write` renders Ruby-to-Rust
//! encoding. Both implement the shared codec walker traits, so the Ruby
//! target spells only the leaves; the walker owns the shape of every tree.

pub mod read;
pub mod write;
