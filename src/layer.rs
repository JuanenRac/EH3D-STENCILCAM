//! Reading a paste layer from the bytes of a file: the size limit and the text that is not UTF-8 are the business of the reader, and what the reader says about the file is
//! shown by the stable code of each notice, so that a notice can be hidden without depending on its wording.

use eh3d_parser_gerber::{parse_bytes_with, GerberFile, InputLimits, Severity};

/// A layer, with what the reader said about it.
#[derive(Debug, Clone)]
pub struct Reading {
    pub file: GerberFile,
    /// The warnings and errors of the reader that were not hidden, one line each, with their code.
    pub shown: Vec<String>,
    /// How many of them are errors: something of the file was lost.
    pub errors: usize,
}

/// Reads the layer. A file larger than `max_bytes` is refused; a notice whose code is in `hide` is left out. Notices of the kind `Info` are never shown.
pub fn read_layer(bytes: &[u8], max_bytes: usize, hide: &[String]) -> Result<Reading, String> {
    let file = parse_bytes_with(bytes, &InputLimits { max_bytes }).map_err(|e| e.to_string())?;
    let (mut shown, mut errors) = (Vec::new(), 0);
    for d in &file.diagnostics {
        if d.severity == Severity::Info || hide.iter().any(|h| h.as_str() == d.code) {
            continue;
        }
        if d.severity == Severity::Error {
            errors += 1;
        }
        let kind = if d.severity == Severity::Error {
            "error"
        } else {
            "warning"
        };
        shown.push(format!(
            "the reader, line {}: {kind}: {} [{}]",
            d.line, d.message, d.code
        ));
    }
    Ok(Reading {
        file,
        shown,
        errors,
    })
}

/// The codes of a `--hide` value (`code,code`), each one checked against the codes the reader has; a code that does not exist is an error, because a misspelt code would hide nothing.
pub fn hidden_codes(value: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for c in value.split(',').map(str::trim).filter(|c| !c.is_empty()) {
        if !eh3d_parser_gerber::codes::ALL.contains(&c) {
            return Err(format!("--hide: '{c}' is not a code of the reader"));
        }
        out.push(c.to_string());
    }
    Ok(out)
}
