#![warn(clippy::missing_docs_in_private_items)]

//! The general "library" code used across all applications living in this cargo/rust application.

/// Exposes functionality for controlling the light firmware.
pub mod lights;

/// Auth0 configuration and helper functionality.
pub mod oauth;

/// Octoprint types and functionality.
pub(crate) mod octoprint;

/// This module contains all of the web/http server types and logic.
pub mod server;
