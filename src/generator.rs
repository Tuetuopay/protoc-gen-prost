//! Main code generator module.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::read_to_string;

use anyhow::{Context, Result};
use prost::Message;
use prost_build::Config;
use prost_types::{compiler::code_generator_response::File, FileDescriptorProto, FileDescriptorSet};
use quote::{format_ident, quote};
use proc_macro2::TokenStream;

/// PROST code generator
///
/// This is a wrapper around the actual `prost_build` generator, but with extras
#[derive(Default)]
pub struct Generator {
    pub config: Config,
    include_file: Option<String>,
    manifest_tpl: Option<String>,
    file_descriptor_set: Option<String>,
}

impl Generator {
    /// Create a new generator from a list of options, as given by protoc directly
    pub fn new_from_opts(opts: Vec<String>) -> (Self, Vec<String>) {
        let (config, opts) = crate::args::config_from_opts(opts);
        let mut this = Self { config, ..Default::default() };
        let mut leftovers = Vec::new();
        let mut include_file = false;

        for opt in opts {
            match opt.splitn(3, '=').collect::<Vec<_>>().as_slice() {
                [] | [""] => (),
                ["include_file"] => include_file = true,
                ["include_file", v] => this.include_file = Some(v.to_string()),
                ["gen_crate"] => this.manifest_tpl = Some("Cargo.toml.tpl".to_owned()),
                ["gen_crate", v] => this.manifest_tpl = Some(v.to_string()),
                ["file_descriptor_set"] => {
                    this.file_descriptor_set = Some("file_descriptor_set.rs".to_owned())
                }
                ["file_descriptor_set", v] => this.file_descriptor_set = Some(v.to_string()),
                _ => leftovers.push(opt),
            }
        }

        // Sane defaults for the include file.
        // When not in crate mode, pick something that can be used out of the box: mod.rs allows the
        // protoc output to be dumped straight in a subfolder of the src/ tree.
        // When in crate mode, well, pick lib.rs as default as it's what makes a lib crate.
        if this.include_file.is_none() {
            if this.manifest_tpl.is_some() {
                this.include_file = Some("lib.rs".to_owned())
            } else if include_file {
                this.include_file = Some("mod.rs".to_owned())
            }
        }

        (this, leftovers)
    }

    /// Generate code for all passed proto files, keyed by the module path
    pub fn generate(mut self, protos: Vec<FileDescriptorProto>) -> Result<Vec<File>> {
        let (manifest, proto_prefix, include_prefix) = match self.manifest_tpl {
            Some(ref path) => {
                let fdesc = self.file_descriptor_set.is_some();
                let toml = gen_manifest(path, &protos, fdesc).context("Cargo.toml gen failed")?;
                let toml = File {
                    name: Some("Cargo.toml".to_owned()),
                    content: Some(toml),
                    ..Default::default()
                };
                (Some(toml), "gen/", "src/")
            }
            None => (None, "", ""),
        };
        let fdesc_set = self.file_descriptor_set.as_ref().map(|name| {
            let set = FileDescriptorSet { file: protos.clone() }.encode_to_vec();
            // So protoc check that the file contents indeed is made of printable characters, so
            // best we can do is to emit a literal slice of bytes...
            let bytes: Vec<_> = set.into_iter().map(|byte| format!("0x{byte:02x}")).collect();
            let line = format!("pub const FILE_DESCRIPTOR_SET: &[u8] = &[{}];", bytes.join(", "));
            File {
                name: Some(format!("{proto_prefix}{name}")),
                content: Some(prettify_str(&line).expect("Prettify file descriptor file failed")),
                ..Default::default()
            }
        });

        let modules = self.config.generate(protos).context("Failed to generate Rust code")?;

        let include_file = self.include_file.as_ref().map(|name| File {
            name: Some(format!("{include_prefix}{name}")),
            content: Some(self.gen_include_file(modules.keys())),
            ..Default::default()
        });

        let files = modules.into_iter().map(|(module, content)| File {
            name: Some(format!("{proto_prefix}{}.rs", module.join("."))),
            content: Some(prettify_str(&content).expect("Prettify generated file failed")),
            ..Default::default()
        });
        let mut files: Vec<_> = files.collect();

        if let Some(include_file) = include_file {
            files.push(include_file);
        }
        if let Some(manifest) = manifest {
            files.push(manifest);
        }
        if let Some(fdesc_set) = fdesc_set {
            files.push(fdesc_set);
        }

        Ok(files)
    }

    fn gen_include_file<'a>(&self, modules: impl Iterator<Item = &'a Vec<String>>) -> String {
        let mut root = Mod::default();
        for module in modules {
            root.push(module);
        }

        let file = root.render(self.manifest_tpl.is_some());
        let desc = self.file_descriptor_set.as_ref().map(|name| match self.manifest_tpl.as_ref() {
            Some(_manifest) => quote! {
                #[cfg(feature = "file_descriptor_set")]
                include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/", #name));
            },
            None => quote! { include!(#name); },
        });

        prettyplease::unparse(&syn::parse2(quote! { #desc #file }).unwrap())
    }
}

fn prettify_str(code: &str) -> Result<String> {
    Ok(prettyplease::unparse(&syn::parse_file(code)?))
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

    fn render(&self, krate: bool) -> TokenStream {
        let include = self.name.clone().map(|name| {
            if krate {
                quote! {
                    #[cfg(feature = #name)]
                    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/", #name, ".rs"));
                }
            } else {
                quote! { include!(concat!(#name, ".rs")); }
            }
        });
        let mods = self.submods.iter().map(|(name, module)| {
            let module = module.render(krate);
            let name = format_ident!("{name}");
            quote! { pub mod #name { #module } }
        });

        quote! { #include #(#mods)* }
    }
}

fn build_deps(protos: &[FileDescriptorProto]) -> BTreeMap<String, BTreeSet<&str>> {
    // Since one proto package can be spread across multiple proto files, we cannot rely on the
    // deps of a single file.
    // We use BTree stuff here since, by nature, they are sorted, which helps us get a stable
    // output (so the generated code is git-friendly).
    let names: BTreeMap<_, _> = protos.iter().map(|file| (file.name(), file.package())).collect();
    let mut deps: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
    for proto in protos {
        let pkg = proto.package();
        let proto_deps = proto.dependency.iter().filter_map(|dep| names.get(dep.as_str()));
        // In some weird cases, protoc includes ourselves in our own dep list...
        let proto_deps = proto_deps.filter(|dep| **dep != pkg);
        // Add them to our list
        deps.entry(pkg.to_owned()).or_default().extend(proto_deps.cloned());
    }
    deps
}

fn gen_manifest(tpl: &str, protos: &[FileDescriptorProto], fdesc: bool) -> Result<String> {
    let tpl = read_to_string(tpl).with_context(|| format!("Read template file {tpl} failed"))?;

    let mut deps = build_deps(protos);
    if fdesc {
        deps.insert("file_descriptor_set".to_owned(), Default::default());
    }

    let deps = deps.into_iter().map(|(feat, deps)| {
        let deps = deps.iter().map(|dep| format!("\"{dep}\"")).collect::<Vec<_>>();
        format!(r#""{feat}" = [{}]"#, deps.join(", "))
    });

    let manifest = tpl.replace("{{ features }}", &deps.collect::<Vec<_>>().join("\n"));

    Ok(manifest)
}
