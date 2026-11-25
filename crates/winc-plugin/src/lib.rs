//! Text-only plugins: validated local packages and bounded Rhai Grain execution.
//!
//! Plugins receive a text snapshot and return a string. The application owns document edits,
//! undo, command registration and shortcuts; scripts never receive editor internals.
#![allow(
    clippy::disallowed_methods,
    reason = "Plugin packages are always local user data"
)]

mod package;
mod runtime;

pub use package::{Catalog, Command, Manifest, Plugin, install, load, set_enabled, uninstall};
pub use runtime::{Input, MAX_TEXT_BYTES};

/// A package validation, installation or script execution error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

#[cfg(test)]
mod tests;
