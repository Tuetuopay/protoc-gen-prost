use anyhow::{bail, Result};
use prost_types::compiler::{code_generator_response::File, CodeGeneratorResponse};
use protoc_gen_prost::{utils::*, Generator};

fn main() {
    let res = match gen_files() {
        Ok(file) => CodeGeneratorResponse { file, ..Default::default() },
        Err(e) => CodeGeneratorResponse { error: Some(format!("{e:?}")), ..Default::default() },
    };
    response_to_env(res).unwrap();
}

fn gen_files() -> Result<Vec<File>> {
    let req = request_from_env()?;

    let (gen, opts) = Generator::new_from_opts(split_escaped(req.parameter(), ','));
    if !opts.is_empty() {
        bail!("Unknown opts:\n - {}", opts.join("\n - "));
    }

    gen.generate(req.proto_file)
}
