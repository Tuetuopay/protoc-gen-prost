//! Main code generator module.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use prost_build::Config;
use prost_types::{compiler::code_generator_response::File, FileDescriptorProto};
use quote::{format_ident, quote};
use proc_macro2::TokenStream;

/// PROST code generator
///
/// This is a wrapper around the actual `prost_build` generator, but with extras
pub struct Generator {
    pub config: Config,
    include_file: Option<String>,
}

impl Generator {
    /// Create a new generator from a list of options, as given by protoc directly
    pub fn new_from_opts(opts: Vec<String>) -> (Self, Vec<String>) {
        let (config, opts) = crate::args::config_from_opts(opts);
        let mut this = Self { config, include_file: None };
        let mut leftovers = Vec::new();

        for opt in opts {
            match opt.splitn(3, '=').collect::<Vec<_>>().as_slice() {
                [] | [""] => (),
                ["include_file", v] => this.include_file = Some(v.to_string()),
                _ => leftovers.push(opt),
            }
        }

        (this, leftovers)
    }

    /// Generate code for all passed proto files, keyed by the module path
    pub fn generate(mut self, protos: Vec<FileDescriptorProto>) -> Result<Vec<File>> {
        let modules = self.config.generate(protos).context("Failed to generate Rust code")?;

        let include_file = self.include_file.map(|name| File {
            name: Some(name),
            content: Some(gen_include_file(modules.keys())),
            ..Default::default()
        });

        let files = modules.into_iter().map(|(module, content)| File {
            name: Some(module.join(".") + ".rs"),
            content: Some(prettyplease::unparse(&syn::parse_file(&content).unwrap())),
            ..Default::default()
        });
        let mut files: Vec<_> = files.collect();

        if let Some(include_file) = include_file {
            files.push(include_file);
        }

        Ok(files)
    }
}

/// Helper structure to build a module tree, tagging each node with the potentially included
/// package file
#[derive(Debug, Default)]
struct Mod {
    /// Submodule list contained by this module. Keyed by module name, not package name.
    submods: BTreeMap<String, Mod>,
    /// The package name to include, if any.
    name: Option<String>,
}

impl Mod {
    /// Push a package to include in the module.
    fn push(&mut self, package: &[String]) {
        if let [name, left @ ..] = package {
            self.add(package.to_owned(), name, left);
        }
    }

    /// Add a package, with the current part where we are.
    /// - `name` is the current module name we are int
    /// - `left` is what's left of the package, i.e. submodules after `name` leading up to the leaf
    ///   module with the actual file import
    fn add(&mut self, package: Vec<String>, name: &str, left: &[String]) {
        let submod = self.submods.entry(name.to_owned()).or_default();
        match left {
            [] => submod.name = Some(package.join(".")),
            [name, left @ ..] => submod.add(package, name, left),
        }
    }

    fn render(&self) -> TokenStream {
        let include = self.name.clone().map(|name| quote! { include!(#name); });
        let mods = self.submods.iter().map(|(name, module)| {
            let name = format_ident!("{name}");
            let module = module.render();
            quote! { pub mod #name { #module } }
        });

        quote! { #include #(#mods)* }
    }
}

fn gen_include_file<'a, M: Iterator<Item = &'a Vec<String>>>(modules: M) -> String {
    let mut root = Mod::default();
    for module in modules {
        root.push(module);
    }
    prettyplease::unparse(&syn::parse2(root.render()).unwrap())
}
