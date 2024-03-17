#![allow(clippy::single_component_path_imports)]

//! Forces dynamic linking of Ens.
//!
//! Dynamic linking causes Bevy to be built and linked as a dynamic library. This will make
//! incremental builds compile much faster.
//!
//! # Warning
//!
//! Do not enable this feature for release builds because this would require you to ship
//! `libstd.so` and `libens_dylib.so` with your game.
//!
//! # Enabling dynamic linking
//!
//! ## The recommended way
//!
//! The easiest way to enable dynamic linking is to use the `--features ens/dynamic_linking` flag when
//! using the `cargo run` command:
//!
//! `cargo run --features ens/dynamic_linking`
//!
//! ## The unrecommended way
//!
//! It is also possible to enable the `dynamic_linking` feature inside of the `Cargo.toml` file. This is
//! unrecommended because it requires you to remove this feature every time you want to create a
//! release build to avoid having to ship additional files with your game.
//!
//! To enable dynamic linking inside of the `Cargo.toml` file add the `dynamic_linking` feature to the
//! ens dependency:
//!
//! `features = ["dynamic_linking"]`
//!
//! ## The manual way
//!
//! Manually enabling dynamic linking is achieved by adding `ens_dylib` as a dependency and
//! adding the following code to the `main.rs` file:
//!
//! ```
//! #[allow(unused_imports)]
//! use ens_dylib;
//! ```
//!
//! It is recommended to disable the `ens_dylib` dependency in release mode by adding the
//! following code to the `use` statement to avoid having to ship additional files with your game:
//!
//! ```
//! #[allow(unused_imports)]
//! #[cfg(debug_assertions)] // new
//! use ens_dylib;
//! ```
