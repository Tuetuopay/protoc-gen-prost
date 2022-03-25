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
