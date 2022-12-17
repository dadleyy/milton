#![forbid(unsafe_code)]
#![warn(clippy::missing_docs_in_private_items)]

//! Provides configuration, discovery and runtime abstractions to split up the responsibility of
//! providing the necessary network entities alexa will look for.

/// A public config definition that can be deserialized.
pub mod config;

/// The discovery module contains the task that listens on a udp socket for alexa and will respond
/// with a payload that it expects, providing "logistical" information about the service.
pub mod discovery;

/// The runtime module contains the "actual" application that binds a tcp socket and receives the
/// actionable requests from alex.
pub mod runtime;
