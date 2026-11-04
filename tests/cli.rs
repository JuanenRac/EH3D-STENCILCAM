use std::fs;
use std::process::Command;

const WITH_POLYGON: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\n%ADD11P,0.8X6*%\nD10*\nX20000Y20000D03*\nD11*\nX40000Y20000D03*\nM02*\n";

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
fn a_layer_with_pads_that_cannot_be_opened_is_not_written_unless_forced() {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_cli_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let layer = dir.join("paste.gtp");
    let out = dir.join("out.nc");
    fs::write(&layer, WITH_POLYGON).unwrap();
    let (layer_s, out_s) = (layer.to_str().unwrap(), out.to_str().unwrap());
    let (ok, why) = run(&["convert", layer_s, out_s, "--thickness", "0.1"]);
    assert!(!ok && why.contains("incomplete"), "{why}");
    assert!(!out.exists(), "nothing is written");
    let (ok, why) = run(&["convert", layer_s, out_s, "--thickness", "0.1", "--force"]);
    assert!(ok, "{why}");
    assert!(out.exists());
    let _ = fs::remove_dir_all(&dir);
}

// ---- the command line, more edges ----

const TWO_PADS: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";

fn run_full(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_more_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_extension_of_the_output_chooses_hpgl_or_gcode_whatever_its_case() {
    let dir = scratch("ext");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let l = layer.to_str().unwrap();
    for (name, starts) in [
        ("a.PLT", "IN;SP1;"),
        ("a.plt", "IN;SP1;"),
        ("a.nc", "G21\n"),
        ("a.txt", "G21\n"),
        ("noext", "G21\n"),
    ] {
        let out = dir.join(name);
        let (c, o, e) = run_full(&["convert", l, out.to_str().unwrap(), "--thickness", "0.1"]);
        assert_eq!(c, Some(0), "{name}: {e}");
        assert!(o.contains("2 openings written"), "{o}");
        let text = fs::read_to_string(&out).unwrap();
        if starts == "G21\n" {
            // G-code carries the standard header first: the generator, its version, the units, the source with its hash and the tools
            assert!(text.starts_with("; generator: eh3d-stencilcam "), "{name}");
            assert!(text.contains("; units: mm (G21)\n"), "{name}");
            assert!(text.contains("; source: paste.gtp sha256:"), "{name}");
            assert!(text.contains("\nG21\nG90\n"), "{name}");
        } else {
            assert!(text.starts_with(starts), "{name}");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn two_runs_write_the_same_bytes_and_mirroring_flips_the_x_of_the_cuts() {
    let dir = scratch("mirror");
    let layer = dir.join("paste.gtp");
    // two pads of different sizes, so that the layer is not its own mirror image
    fs::write(&layer, "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\n%ADD11R,2.0X0.5*%\nD10*\nX20000Y20000D03*\nD11*\nX80000Y20000D03*\nM02*\n").unwrap();
    let l = layer.to_str().unwrap();
    let (a, b, m) = (dir.join("a.nc"), dir.join("b.nc"), dir.join("m.nc"));
    run_full(&["convert", l, a.to_str().unwrap(), "--thickness", "0.1"]);
    run_full(&["convert", l, b.to_str().unwrap(), "--thickness", "0.1"]);
    run_full(&[
        "convert",
        l,
        m.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--mirror",
    ]);
    assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
    // mirroring moves the cuts but keeps the extent of the layer along x
    let extent = |p: &std::path::Path| -> (f64, f64) {
        let t = fs::read_to_string(p).unwrap();
        let xs: Vec<f64> = t
            .lines()
            .filter(|l| l.starts_with("G0 X") || l.starts_with("G1 X"))
            .map(|l| l[4..].split(' ').next().unwrap().parse::<f64>().unwrap())
            .collect();
        (
            xs.iter().cloned().fold(f64::INFINITY, f64::min),
            xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let (ea, em) = (extent(&a), extent(&m));
    assert!(
        (ea.0 - em.0).abs() < 0.1 && (ea.1 - em.1).abs() < 0.1,
        "{ea:?} {em:?}"
    );
    assert_ne!(fs::read(&a).unwrap(), fs::read(&m).unwrap());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn convert_writes_nothing_when_an_opening_will_not_release_the_paste_unless_forced() {
    let dir = scratch("weak");
    let layer = dir.join("paste.gtp");
    fs::write(
        &layer,
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,0.3X0.3*%\nD10*\nX20000Y20000D03*\nM02*\n",
    )
    .unwrap();
    let out = dir.join("out.nc");
    let (l, o) = (layer.to_str().unwrap(), out.to_str().unwrap());
    let (c, _, e) = run_full(&["convert", l, o, "--thickness", "0.2", "--kerf", "0"]);
    assert_eq!(c, Some(1));
    assert!(
        e.contains("will not release the paste") && e.contains("opening 1"),
        "{e}"
    );
    assert!(!out.exists());
    let (c, _, e) = run_full(&["check", l, "--thickness", "0.2"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("1 openings will not release"), "{e}");
    assert_eq!(
        run_full(&[
            "convert",
            l,
            o,
            "--thickness",
            "0.2",
            "--kerf",
            "0",
            "--force"
        ])
        .0,
        Some(0)
    );
    assert!(out.exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn convert_writes_nothing_when_the_kerf_eats_an_opening_even_with_force() {
    let dir = scratch("kerf");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let out = dir.join("out.nc");
    let (c, _, e) = run_full(&[
        "convert",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--kerf",
        "0.9",
        "--thickness",
        "0.05",
        "--force",
    ]);
    assert_eq!(c, Some(1));
    assert!(e.contains("is gone"), "{e}");
    assert!(!out.exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_numbers_of_the_command_line_are_checked_before_anything_is_written() {
    let dir = scratch("numbers");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let l = layer.to_str().unwrap();
    let out = dir.join("out.nc");
    let o = out.to_str().unwrap();
    for (extra, word) in [
        (vec!["--smax", "0"], "--smax"),
        (vec!["--smax", "100001"], "--smax"),
        (vec!["--feed", "0"], "feed"),
        (vec!["--feed", "30001"], "feed"),
        (vec!["--feed", "fast"], "--feed"),
        (vec!["--material", "wood"], "wood"),
        (vec!["--thickness", "5"], "thickness_mm"),
        (vec!["--shrink", "-1"], "shrink_percent"),
        (vec!["--radius", "nan"], "--radius"),
        (vec!["--kerf"], "needs a value"),
    ] {
        let mut args = vec!["convert", l, o];
        args.extend(extra.iter());
        let (c, _, e) = run_full(&args);
        assert_eq!(c, Some(1), "{extra:?}");
        assert!(e.contains(word), "{extra:?}: {e}");
    }
    assert!(!out.exists());
    // a feed of exactly 30000 and a power of exactly 100000 are the largest accepted
    assert_eq!(
        run_full(&[
            "convert",
            l,
            o,
            "--feed",
            "30000",
            "--smax",
            "100000",
            "--thickness",
            "0.1"
        ])
        .0,
        Some(0)
    );
    assert!(fs::read_to_string(&out).unwrap().contains("M3 S100000"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_layer_and_an_output_in_a_missing_folder_name_their_paths() {
    let dir = scratch("paths");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let (c, _, e) = run_full(&["check", dir.join("none.gtp").to_str().unwrap()]);
    assert_eq!(c, Some(1));
    assert!(e.contains("none.gtp"), "{e}");
    let missing = dir.join("no").join("such").join("out.nc");
    let (c, _, e) = run_full(&[
        "convert",
        layer.to_str().unwrap(),
        missing.to_str().unwrap(),
        "--thickness",
        "0.1",
    ]);
    assert_eq!(c, Some(1));
    assert!(e.contains("out.nc"), "{e}");
    assert_eq!(
        run_full(&["convert", layer.to_str().unwrap()]).0,
        Some(1),
        "an output file is needed"
    );
    assert_eq!(run_full(&["check"]).0, Some(1), "a layer is needed");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn check_reports_the_count_the_thickness_and_the_smallest_ratios() {
    let dir = scratch("check");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let (c, o, e) = run_full(&[
        "check",
        layer.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--shrink",
        "0",
    ]);
    assert_eq!(c, Some(0), "{e}");
    assert!(
        o.contains("2 openings; thickness 0.1 mm, shrink 0 %"),
        "{o}"
    );
    assert!(
        o.contains("smallest aspect ratio 5.00") && o.contains("smallest area ratio 1.67"),
        "{o}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_layer_with_no_pads_and_a_file_that_is_not_gerber_do_not_crash() {
    let dir = scratch("empty");
    let (empty, junk) = (dir.join("empty.gtp"), dir.join("junk.gtp"));
    fs::write(&empty, "%FSLAX24Y24*%\n%MOMM*%\nM02*\n").unwrap();
    fs::write(&junk, [0u8, 159, 146, 150, 255, 254, 1, 2, 3]).unwrap();
    for f in [&empty, &junk] {
        let (c, _, _) = run_full(&["check", f.to_str().unwrap()]);
        assert!(matches!(c, Some(0) | Some(1)), "{f:?}: {c:?}");
        let out = dir.join("o.nc");
        let (c, _, _) = run_full(&["convert", f.to_str().unwrap(), out.to_str().unwrap()]);
        assert!(matches!(c, Some(0) | Some(1)), "{f:?}: {c:?}");
    }
    let _ = fs::remove_dir_all(&dir);
}
