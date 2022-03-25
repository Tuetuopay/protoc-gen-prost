use std::io::{Read, Write};

use anyhow::{Result, Context, bail};
use prost::Message;
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse, code_generator_response::File};

mod args;
mod utils;

fn main() {
    let res = match gen_files() {
        Ok(file) => CodeGeneratorResponse { file, ..Default::default() },
        Err(e) => CodeGeneratorResponse { error: Some(format!("{e:?}")), ..Default::default() },
    };

    let mut buf = Vec::new();
    res.encode(&mut buf).expect("Failed to serialize response");
    std::io::stdout().write_all(&buf).expect("Failed to write response to stdout");
}

fn gen_files() -> Result<Vec<File>> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).context("Failed to read stdin")?;

    let req = match CodeGeneratorRequest::decode(buf.as_slice()) {
        Ok(req) => req,
        Err(e) => bail!("Failed to decode CodeGeneratorRequest: {e:?}"),
    };

    let (mut config, opts) = args::config_from_opts(utils::split_escaped(req.parameter(), ','));
    if !opts.is_empty() {
        bail!("Unknown opts:\n - {}", opts.join("\n - "));
    }

    let modules = config.generate(req.proto_file).context("Failed to generate Rust code")?;
    let files = modules.into_iter().map(|(module, content)| File {
        name: Some(module.join(".") + ".rs"),
        content: Some(content),
        ..Default::default()
    });

    Ok(files.collect())
}
