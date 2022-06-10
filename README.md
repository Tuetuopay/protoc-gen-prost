THIS IS NOT THE PLUGIN PUBLISHED ON CRATES.IO.

If you installed the plugin through `cargo install protoc-gen-prost`, then the
plugin you installed is the one from https://github.com/neoeinstein/protoc-gen-prost.

This repository is kept for visibility and because some people depend on it.

# protoc-gen-prost

A `protoc` plugin to generate PROST! code.

While the recommended way to use `prost` in Rust projects is with `prost-build`
and running `protoc` through Cargo and `build.rs`, a  protoc plugin allows to
get a standard workflow in the protobuf ecosystem. Also, precompiling proto
files to Rust code as files has some advantages:

- easier to share compiled code across multiple projects, since they don't all
  need to setup the prost build
- rust code can be stored in an easy to browse fashion (git, ...)
- integrates well with standard protobuf/grpc/... tooling
- compatible with `buf`

## Usage

All usage examples assume `protoc-gen-prost` was installed and can be found by
`protoc` (i.e. in your `$PATH`), and the output directory was created. The
example proto files can be found in this repository under `proto/`.

### Basic usage

```bash
protoc --prost_out=hello-rs -I proto types.proto
```

The proto file ends up compiled, as expected:

```text
hello-rs/
└── helloworld.rs
```

And its compiled contents:

```rust
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    #[prost(string, tag = "1")]
    pub say: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(string, tag = "1")]
    pub say: ::prost::alloc::string::String,
}
```

### Specifying flags

Options can be passed to the plugin through `protoc`, using the `--prost_opt`
flag, separated with commas.

- Boolean options can just be specified
- Options taking a value are in the form `opt=value`
- Options taking a key and a value are in the form `opt=key=value`
- Commas can be escaped with `\`
- Some options can be specified multiple times. Please refer to the option list
  below for a full reference.
- Some options have a default value

_note: `protoc-gen-prost` ignores whitespaces around commas to allow the opt
string to be spread on multiple lines_

```bash
protoc \
  -I proto \
  --prost_out=hello-rs \
  --prost_opt="
    type_attribute=.helloworld.Message=#[derive(::serde::Serialize\, ::serde::Deserialize)],
    type_attribute=.helloworld.Message=#[serde(rename_all = \"kebab-case\")],
    field_attribute=.helloworld.Message.say=#[serde(rename = \"speak\")]" \
  types.proto
```

The above sets two
[type attributes](https://docs.rs/prost-build/0.9.0/prost_build/struct.Config.html#method.type_attribute)
and a
[field attribute](https://docs.rs/prost-build/0.9.0/prost_build/struct.Config.html#method.field_attribute)
on a generated struct, and results in the following in `hello-rs/helloworld.rs`:

```rust
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    #[prost(string, tag = "1")]
    #[serde(rename = "speak")]
    pub say: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    #[prost(string, tag = "1")]
    pub say: ::prost::alloc::string::String,
}
```

### Module / include file generation

To ease usage and not need to `include!` each and every generated file, the
`include_file` option can be used. By default, it will generate a `mod.rs` file
including all compiled Rust files, in a Rust module hierachy matching the proto
package hierarchy:

```bash
protoc -I proto --prost_out=hello-rs --prost_opt=include_file types.proto foo.proto
```

Results in the following hierarchy:

```text
hello-rs/
├── helloworld.foo.bar.rs
├── helloworld.rs
└── mod.rs
```

... and with a complete include file:

```rust
pub mod helloworld {
    include!(concat!("helloworld", ".rs"));
    pub mod foo {
        pub mod bar {
            include!(concat!("helloworld.foo.bar", ".rs"));
        }
    }
}
```

The option can optionally take a file name to have it named something other than
`mod.rs`:

```bash
protoc -I proto --prost_out=hello-rs --prost_opt=include_file=proto.rs \
  types.proto foo.proto
```

### Crate generation

For larger proto codebases, it is better to generate a full crate, or just to
shre the compiled results: it can be put in a crate and used as a direct Cargo
dependency, no fiddling with `build.rs` or `protoc` for the end-user.

Additionally, the generated crate will sport a feature flag for each proto
package (with dependency resolution) to avoid slowing down build when only a few
packages are used at once.

Its only requirement is a `Cargo.toml` template to be put in the generated
crate. It is required to insert feature flags in it (`{{ features }}` will get
replaced by all features):

```toml
[package]
name = "hello-rs"
version = "1.0.0"
edition = "2021"

[dependencies]
prost = "0.9"
prost-types = "0.9"

[features]
{{ features }}
```

As for `input_file`, the `gen_crate` argument takes an optional value,
defaulting to `Cargo.toml.tpl` (path relative to `protoc`'s working directory'):

```bash
protoc -I proto \
  --prost_out=hello-rs \
  --prost_opt=gen_crate=hello-rs/Cargo.toml.tpl \
  types.proto foo.proto
```

Results in the full crate:

```text
hello-rs
├── Cargo.toml
├── Cargo.toml.tpl
├── gen
│   ├── helloworld.foo.bar.rs
│   └── helloworld.rs
└── src
    └── lib.rs
```

with the following `lib.rs` and `Cargo.toml`:

```rust
pub mod helloworld {
    #[cfg(feature = "helloworld")]
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/", "helloworld", ".rs"));
    pub mod foo {
        pub mod bar {
            #[cfg(feature = "helloworld.foo.bar")]
            include!(
                concat!(env!("CARGO_MANIFEST_DIR"), "/gen/", "helloworld.foo.bar", ".rs")
            );
        }
    }
}
```

```toml
[package]
name = "hello-grpc"
version = "1.0.0"
edition = "2021"

[dependencies]
prost = "0.9"
prost-types = "0.9"

[features]
"helloworld" = ["helloworld.foo.bar"]
"helloworld.foo.bar" = []
```

## Options reference

### Options inherited from `prost_build`

Please refer to
[`prost_build`'s documentation](https://docs.rs/prost-build/0.9/prost_build/struct.Config.html#method.bytes)
for a reference of each option, as those are 1:1 mappings.

- `btree_map=<value>`: repeatable
- `bytes=<value>`: repeatable
- `compile_well_known_types`: boolean
- `default_package_filename=<value>`: string
- `disable_comments=<value>`: repeatable
- `extern_path=<key>=<value>`: repeatable
- `field_attribute=<key>=<value>`: repeatable
- `retain_enum_prefix`: boolean
- `type_attribute=<key>=<value>`: repeatable

### Options specific to the plugin

- `include_file[=<value>]`: output a Rust file named `<value>` that will include
  all generated `.rs` files.
  * Defaults to `mod.rs` _without_ `gen_crate`, else to `lib.rs`
  * Integrates with `gen_crate`: specifying the `<value>` overrides the default
    `lib.rs` name. This is useful if more code needs to be put in the generated
    crate.
- `gen_crate[=<value>]`: generate a full-blown Rust crate with a manifest based
  on a template at `<value>`, relative to `protoc`'s working directory.
  * Defaults to `Cargo.toml.tpl`
  * Implies `include_file` set to `lib.rs` if not already set
- `file_descriptor_set[=<value>]`: output the protobuf `FileDescriptorSet` from
  `protoc`. Used for plugins, reflection, ...
  * Defaults to `file_descriptor_set.rs`
  * It will contain a static `FILE_DESCRIPTOR_SET` variable containing a byte
    array of the serialized message.

## Integration with other tools

`protoc-gen-prost` is compatible with e.g. [Buf](https://buf.build). Please note
that for the full crate generation (`gen_crate` option) or include file
generation, the default buf strategy does not work, and requires the `all`
strategy for the include file to be complete.
