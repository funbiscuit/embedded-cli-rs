#![warn(rust_2018_idioms, missing_debug_implementations)]
#![no_std]

// std used for simpler testing
#[cfg(test)]
extern crate std;

pub mod arguments;
pub mod autocomplete;
pub mod buffer;
mod builder;
pub mod cli;
pub mod codes;
pub mod command;
mod editor;
pub mod help;
mod history;
mod input;
pub mod service;
mod token;
mod utf8;
mod utils;
pub mod writer;

/// Macro available if embedded-cli is built with `features = ["macros"]`.
#[cfg(feature = "embedded-cli-macros")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "macros")))]
pub use embedded_cli_macros::{Command, CommandGroup};

// Used by generated code. Not public API.
#[doc(hidden)]
#[path = "private/mod.rs"]
pub mod __private;

//TODO: organize pub uses better
