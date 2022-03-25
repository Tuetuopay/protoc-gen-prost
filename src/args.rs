//! Options for the protoc plugin
//!
//! Set with protoc's `--prost_opt`

use prost_build::Config;

/// Take a list of arguments, in the form of key=value, and returns the leftovers arguments
pub fn config_from_opts(opts: Vec<String>) -> (Config, Vec<String>) {
    let mut config = Config::new();
    let mut leftovers = Vec::new();

    let mut map_types = Vec::new();
    let mut byte_types = Vec::new();
    let mut disable_comments = Vec::new();

    for opt in opts {
        match opt.splitn(3, '=').collect::<Vec<_>>().as_slice() {
            [] | [""] => (),
            ["btree_map", v] => map_types.push(v.to_string()),
            ["bytes", v] => byte_types.push(v.to_string()),
            ["compile_well_known_types"] => { config.compile_well_known_types(); }
            ["default_package_filename", v] => { config.default_package_filename(*v); }
            ["disable_comments", v] => disable_comments.push(v.to_string()),
            ["extern_path", k, v] => { config.extern_path(*k, *v); },
            ["field_attribute", k, v] => { config.field_attribute(k, v); },
            ["file_descriptor_set", v] => { config.file_descriptor_set_path(v); }
            ["include_file", v] => { config.include_file(v); }
            ["retain_enum_prefix"] => { config.retain_enum_prefix(); }
            ["type_attribute", k, v] => { config.type_attribute(k, v); },
            _ => leftovers.push(opt),
        }
    }

    config.btree_map(map_types).bytes(byte_types).disable_comments(disable_comments);

    (config, leftovers)
}
