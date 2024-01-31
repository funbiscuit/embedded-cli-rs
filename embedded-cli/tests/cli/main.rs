#![warn(rust_2018_idioms)]

#[cfg(feature = "autocomplete")]
mod autocomplete;
#[cfg(not(feature = "autocomplete"))]
mod autocomplete_disabled;
mod base;
#[cfg(feature = "history")]
mod history;
#[cfg(not(feature = "history"))]
mod history_disabled;
mod options;
mod subcommand;
mod terminal;
mod wrapper;
mod writer;
