//! Ens's dynamic plugin loading functionality.
//!
//! This crate allows loading dynamic libraries (`.dylib`, `.so`) that export a single
//! [`Plugin`](ens_app::Plugin). For usage, see [`crate::loader::load_plugin`].
//!
//! Note that dynamic linking and loading is inherently unsafe because it allows executing foreign
//! code. Additionally, Rust does not have a stable ABI and may produce
//! incompatible libraries across Rust versions, or even subsequent compilations. This will not work
//! well in scenarios such as modding, but can work if the dynamic plugins and the main app are
//! built at the same time, such as with Downloadable Content (DLC) packs.
//!
//! Consider using [`stabby`]: https://github.com/ZettaScaleLabs/stabby

mod loader;

pub use loader::*;
