//! # protoc-gen-prost
//!
//! Library backing the protoc plugin for prost.
//!
//! If you are looking to use the plugin, look for the binary. If, however, you want to write a
//! protoc plugin that uses prost (like a gRPC layer using Prost), you are at the right place.
//!
//! Look at the binary implementation for more usage example.

mod args;
mod generator;
mod utils;

pub use generator::Generator;
pub use utils::split_escaped;
