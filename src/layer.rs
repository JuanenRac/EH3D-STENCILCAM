//! Reading a paste layer from the bytes of a file: the size limit and the text that is not UTF-8 are the business of the reader, and what the reader says about the file is
//! shown by the stable code of each notice, so that a notice can be hidden without depending on its wording.

use eh3d_parser_gerber::{parse_bytes_with, GerberFile, InputLimits, Severity};

/// The language of the person from the variables of the system (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`); English when it is none of them.
pub fn language() -> &'static str {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            if v.is_empty() {
                continue;
            }
            let code = v.chars().take(2).collect::<String>().to_ascii_lowercase();
            return ["es", "de", "fr", "it", "ja", "zh", "pt"]
                .into_iter()
                .find(|l| *l == code)
                .unwrap_or("en");
        }
    }
    "en"
}

/// The message of a file that is over the size limit, in `lang` (one of the eight languages of the suite; English for any other).
pub fn too_large_message(size: usize, limit: usize, lang: &str) -> String {
    let human = |n: usize| {
        if n >= 1_048_576 {
            format!("{} MiB", n / 1_048_576)
        } else {
            format!("{n} bytes")
        }
    };
    let (s, l) = (human(size), human(limit));
    match lang {
        "es" => format!("el fichero supera el límite ({s} frente a {l}); use --max-bytes si es correcto"),
        "de" => format!("die Datei überschreitet die Grenze ({s} gegenüber {l}); --max-bytes verwenden, wenn das richtig ist"),
        "fr" => format!("le fichier dépasse la limite ({s} contre {l}) ; utilisez --max-bytes si c'est voulu"),
        "it" => format!("il file supera il limite ({s} contro {l}); usare --max-bytes se è corretto"),
        "ja" => format!("ファイルが上限を超えています（{s}、上限 {l}）。正しい場合は --max-bytes を使ってください"),
        "zh" => format!("文件超过上限（{s}，上限 {l}）；如无误，请使用 --max-bytes"),
        "pt" => format!("o ficheiro excede o limite ({s} contra {l}); use --max-bytes se estiver correto"),
        _ => format!("the file is above the limit ({s} against {l}); use --max-bytes if that is right"),
    }
}

/// Reads a layer file, refusing it before it is read into memory when it is larger than `max_bytes`; the refusal is written in `lang`.
pub fn read_layer_file(path: &str, max_bytes: usize, lang: &str) -> Result<Vec<u8>, String> {
    use eh3d_parser_gerber::{read_file_limited, ReadError};
    read_file_limited(std::path::Path::new(path), &InputLimits { max_bytes }).map_err(|e| match e {
        ReadError::TooLarge(eh3d_parser_gerber::InputError::TooLarge { size, limit }) => {
            too_large_message(size, limit, lang)
        }
        other => other.to_string(),
    })
}

/// A layer, with what the reader said about it.
#[derive(Debug, Clone)]
pub struct Reading {
    pub file: GerberFile,
    /// The warnings and errors of the reader that were not hidden, one line each, with their code.
    pub shown: Vec<String>,
    /// How many errors the reader found, hidden or not: something of the file was lost.
    pub errors: usize,
}

/// Reads the layer. A file larger than `max_bytes` is refused; a notice whose code is in `hide` is left out. Notices of the kind `Info` are never shown.
pub fn read_layer(bytes: &[u8], max_bytes: usize, hide: &[String]) -> Result<Reading, String> {
    let file = parse_bytes_with(bytes, &InputLimits { max_bytes }).map_err(|e| e.to_string())?;
    let (mut shown, mut errors) = (Vec::new(), 0);
    for d in &file.diagnostics {
        if d.severity == Severity::Info {
            continue;
        }
        // hiding a notice only keeps it off the screen: an error that is hidden still counts, because what it lost is lost
        if d.severity == Severity::Error {
            errors += 1;
        }
        if hide.iter().any(|h| h.as_str() == d.code) {
            continue;
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
