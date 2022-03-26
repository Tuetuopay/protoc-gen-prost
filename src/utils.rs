use std::io::{Read, Write};

use anyhow::{anyhow, Result, Context, bail};
use prost::Message;
use prost_types::compiler::{CodeGeneratorRequest, CodeGeneratorResponse};

/// Split a string by a char separator, but not when the separator is preceded by a `\`
pub fn split_escaped(string: &str, sep: char) -> Vec<String> {
    let mut ret = Vec::new();
    let mut full_substr = String::new();

    for substr in string.split(sep) {
        if let Some(substr) = substr.strip_suffix('\\') {
            full_substr.push_str(substr);
            full_substr.push(sep);
        } else {
            ret.push(full_substr + substr);
            full_substr = String::new();
        }
    }

    ret
}

/// Parse the request from protoc, passed in stdin
pub fn request_from_env() -> Result<CodeGeneratorRequest> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).context("Failed to read stdin")?;

    CodeGeneratorRequest::decode(buf.as_slice()).map_err(|e| {
        anyhow!("Failed to decode CodeGeneratorRequest: {e:?}")
    })
}

/// Write the response to protoc, through stdout
pub fn response_to_env(res: CodeGeneratorResponse) -> Result<()> {
    let mut buf = Vec::new();
    res.encode(&mut buf).map_err(|e| anyhow!("Failed to serialize response: {e:?}"))?;
    std::io::stdout().write_all(&buf).context("Failed to write response to stdout")?;
    Ok(())
}
