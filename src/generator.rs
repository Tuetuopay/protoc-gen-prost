//! Main code generator module.

use anyhow::{Context, Result};
use prost_build::Config;
use prost_types::{FileDescriptorProto, compiler::code_generator_response::File};

/// PROST code generator
///
/// This is a wrapper around the actual `prost_build` generator, but with extras
pub struct Generator {
    pub config: Config,
}

impl Generator {
    /// Create a new generator from a list of options, as given by protoc directly
    pub fn new_from_opts(opts: Vec<String>) -> (Self, Vec<String>) {
        let (config, leftovers) = crate::args::config_from_opts(opts);
        (Self { config }, leftovers)
    }

    /// Generate code for all passed proto files, keyed by the module path
    pub fn generate(mut self, protos: Vec<FileDescriptorProto>) -> Result<Vec<File>> {
        let modules = self.config.generate(protos).context("Failed to generate Rust code")?;
        let files = modules.into_iter().map(|(module, content)| File {
            name: Some(module.join(".") + ".rs"),
            content: Some(content),
            ..Default::default()
        });

        Ok(files.collect())
    }
}
