use eh3d_stencilcam::layer::{hidden_codes, read_layer};
use std::fs;
use std::process::Command;

const GOOD: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nM02*\n";
const NO_UNITS: &[u8] = b"%FSLAX24Y24*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nM02*\n";

fn latin1() -> Vec<u8> {
    // a comment with an e with an acute accent written as one byte (Latin-1), which is not UTF-8
    let mut b = b"G04 Capa de pasta, fabricaci".to_vec();
    b.push(0xF3);
    b.extend_from_slice(b"n*\n");
    b.extend_from_slice(GOOD.as_bytes());
    b
}

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn a_latin1_file_is_read_and_the_notice_carries_its_code() {
    let r = read_layer(&latin1(), 1 << 20, &[]).unwrap();
    assert_eq!(r.errors, 0);
    assert!(
        r.shown.iter().any(|l| l.contains("[input-not-utf8]")),
        "{:?}",
        r.shown
    );
    assert_eq!(r.file.apertures.len(), 1);
}

#[test]
fn a_notice_is_hidden_by_its_code_not_by_its_text() {
    let hide = hidden_codes("input-not-utf8").unwrap();
    let r = read_layer(&latin1(), 1 << 20, &hide).unwrap();
    assert!(r.shown.is_empty(), "{:?}", r.shown);
    // the other notices stay
    let r = read_layer(NO_UNITS, 1 << 20, &hide).unwrap();
    assert!(
        r.shown.iter().any(|l| l.contains("[units-missing]")),
        "{:?}",
        r.shown
    );
    let r = read_layer(NO_UNITS, 1 << 20, &hidden_codes("units-missing").unwrap()).unwrap();
    assert!(r.shown.is_empty());
}

#[test]
fn a_misspelt_code_is_an_error_because_it_would_hide_nothing() {
    assert!(hidden_codes("units-mising").is_err());
    assert_eq!(
        hidden_codes("units-missing, input-not-utf8,")
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn a_file_over_the_limit_is_refused_before_it_is_read() {
    let e = read_layer(GOOD.as_bytes(), 10, &[]).unwrap_err();
    assert!(e.contains("above the limit"), "{e}");
}

#[test]
fn an_error_of_the_reader_is_counted() {
    let r = read_layer(b"%FSLAX24Y24*%\n%MOMM*%\nG36*\nM02*\n", 1 << 20, &[]).unwrap();
    assert!(r.errors >= 1, "{:?}", r.shown);
}

#[test]
fn the_command_line_applies_the_limit_and_the_filter() {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_read_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let layer = dir.join("paste.gtp");
    fs::write(&layer, NO_UNITS).unwrap();
    let l = layer.to_str().unwrap();
    let (ok, why) = run(&["check", l, "--thickness", "0.1", "--max-bytes", "10"]);
    assert!(!ok && why.contains("above the limit"), "{why}");
    let (_, why) = run(&["check", l, "--thickness", "0.1"]);
    assert!(why.contains("[units-missing]"), "{why}");
    let (_, why) = run(&["check", l, "--thickness", "0.1", "--hide", "units-missing"]);
    assert!(!why.contains("units-missing"), "{why}");
    let (ok, why) = run(&["check", l, "--hide", "no-such-code"]);
    assert!(!ok && why.contains("not a code"), "{why}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_hidden_error_still_counts_as_an_error() {
    let text = b"%FSLAX24Y24*%\n%MOMM*%\nG36*\nM02*\n";
    let shown = read_layer(text, 1 << 20, &[]).unwrap();
    assert!(shown.errors >= 1);
    let code = shown
        .shown
        .iter()
        .find(|l| l.contains(": error:"))
        .and_then(|l| l.rsplit('[').next())
        .map(|c| c.trim_end_matches(']').to_string())
        .expect("an error with a code");
    let hidden = read_layer(text, 1 << 20, &hidden_codes(&code).unwrap()).unwrap();
    assert_eq!(hidden.errors, shown.errors);
    assert!(hidden
        .shown
        .iter()
        .all(|l| !l.contains(&format!("[{code}]"))));
}

#[test]
fn the_refusal_of_a_file_over_the_limit_is_written_in_the_eight_languages() {
    use eh3d_stencilcam::layer::{read_layer_file, too_large_message};
    for (lang, word) in [
        ("en", "above the limit"),
        ("es", "supera el límite"),
        ("de", "überschreitet die Grenze"),
        ("fr", "dépasse la limite"),
        ("it", "supera il limite"),
        ("ja", "上限を超えています"),
        ("zh", "文件超过上限"),
        ("pt", "excede o limite"),
    ] {
        let m = too_large_message(5_000_000, 1_048_576, lang);
        assert!(
            m.contains(word) && m.contains("4 MiB") && m.contains("1 MiB"),
            "{lang}: {m}"
        );
    }
    // the file is refused by its size before it is read
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_limit_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let f = dir.join("big.gtp");
    fs::write(&f, GOOD).unwrap();
    let e = read_layer_file(f.to_str().unwrap(), 10, "es").unwrap_err();
    assert!(e.contains("supera el límite"), "{e}");
    assert!(read_layer_file(f.to_str().unwrap(), 1 << 20, "es").is_ok());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_arc_tolerance_of_the_command_line_is_checked() {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_arc_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let layer = dir.join("paste.gtp");
    fs::write(&layer, GOOD).unwrap();
    let l = layer.to_str().unwrap();
    for bad in ["0", "5", "nan", "x"] {
        let (ok, why) = run(&["check", l, "--thickness", "0.1", "--arc-tolerance", bad]);
        assert!(!ok && why.contains("--arc-tolerance"), "{bad}: {why}");
    }
    let (_, why) = run(&["check", l, "--thickness", "0.1", "--arc-tolerance", "0.002"]);
    assert!(!why.contains("--arc-tolerance"), "{why}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn convert_never_overwrites_the_layer_with_the_stencil() {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_same_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let layer = dir.join("paste.gtp");
    fs::write(&layer, GOOD).unwrap();
    let l = layer.to_str().unwrap();
    let (ok, why) = run(&["convert", l, l, "--force"]);
    assert!(!ok && why.contains("also the layer"), "{why}");
    assert_eq!(fs::read_to_string(&layer).unwrap(), GOOD);
    let _ = fs::remove_dir_all(&dir);
}
