//! The drawing of the openings of a stencil, the counterpart of the drawing that is read: circles and rectangles in millimetres that read back as the same openings, the same text for the
//! same openings, an obround opening refused, and a file that exists never replaced unless it is asked.

use eh3d_stencilcam::dxf_in::from_dxf;
use eh3d_stencilcam::dxf_out::{to_dxf, write_new, DXF_VERSION};
use eh3d_stencilcam::openings::{Form, Opening};
use std::fs;
use std::process::Command;

fn open(x: f64, y: f64, w: f64, h: f64, form: Form) -> Opening {
    Opening { x, y, w, h, form }
}

struct Seed(u64);

impl Seed {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    /// A number from `low` to `high` with three decimals.
    fn between(&mut self, low: f64, high: f64) -> f64 {
        let steps = ((high - low) * 1000.0) as u64;
        low + (self.next() % (steps + 1)) as f64 / 1000.0
    }
}

fn same(a: &Opening, b: &Opening) -> bool {
    a.form == b.form
        && (a.x - b.x).abs() < 1e-9
        && (a.y - b.y).abs() < 1e-9
        && (a.w - b.w).abs() < 1e-9
        && (a.h - b.h).abs() < 1e-9
}

#[test]
fn the_drawing_of_circles_and_rectangles_reads_back_as_the_same_openings() {
    let list = [
        open(2.0, 2.0, 0.8, 0.8, Form::Round),
        open(8.5, 3.25, 1.0, 0.5, Form::Rectangle),
        open(-4.0, 0.125, 0.3, 1.2, Form::Rectangle),
    ];
    let (text, left_out) = to_dxf(&list, false).unwrap();
    assert_eq!(left_out, 0);
    let (back, notes) = from_dxf(&text).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(back.len(), 3);
    for (a, b) in list.iter().zip(&back) {
        assert!(same(a, b), "{a:?} {b:?}");
    }
    assert!(text.contains(DXF_VERSION));
}

#[test]
fn the_text_is_the_same_for_the_same_openings_and_names_millimetres() {
    let list = [
        open(1.5, 2.0, 0.5, 0.5, Form::Round),
        open(4.0, 2.0, 1.0, 0.5, Form::Rectangle),
    ];
    let (a, _) = to_dxf(&list, false).unwrap();
    let (b, _) = to_dxf(&list, false).unwrap();
    assert_eq!(a, b);
    // the header names the format and the unit of the drawing (4 is millimetres) and nothing else of the moment it was written
    assert!(a.starts_with("  0\nSECTION\n  2\nHEADER\n  9\n$ACADVER\n  1\nAC1014\n  9\n$INSUNITS\n 70\n4\n  0\nENDSEC\n"), "{a}");
    assert!(a.ends_with("  0\nENDSEC\n  0\nEOF\n"));
    assert_eq!(a.matches("CIRCLE").count(), 1);
    assert_eq!(a.matches("LWPOLYLINE").count(), 1);
    // the circle: centre and radius (half of the width)
    assert!(a.contains(" 10\n1.5\n 20\n2\n 30\n0\n 40\n0.25\n"), "{a}");
    // the rectangle: four corners round the centre, closed
    assert!(a.contains(" 90\n4\n 70\n1\n 10\n3.5\n 20\n1.75\n 10\n4.5\n 20\n1.75\n 10\n4.5\n 20\n2.25\n 10\n3.5\n 20\n2.25\n"), "{a}");
    assert!(!a.contains('\r'), "one kind of line end");
}

#[test]
fn an_obround_opening_is_refused_unless_it_is_left_out_and_nothing_to_draw_is_an_error() {
    let list = [
        open(1.0, 1.0, 0.5, 0.5, Form::Round),
        open(3.0, 1.0, 1.0, 0.4, Form::Obround),
    ];
    let e = to_dxf(&list, false).unwrap_err();
    assert!(e.contains("1 obround opening(s) cannot be drawn"), "{e}");
    let (text, left_out) = to_dxf(&list, true).unwrap();
    assert_eq!(left_out, 1);
    assert_eq!(from_dxf(&text).unwrap().0.len(), 1);
    assert!(to_dxf(&list[1..], true)
        .unwrap_err()
        .contains("no opening to draw"));
    assert!(to_dxf(&[], false).is_err());
    let bad = [open(f64::NAN, 0.0, 1.0, 1.0, Form::Round)];
    assert!(to_dxf(&bad, false)
        .unwrap_err()
        .contains("no size or place"));
    let bad = [open(0.0, 0.0, 0.0, 1.0, Form::Rectangle)];
    assert!(to_dxf(&bad, false).is_err());
}

#[test]
fn a_seeded_set_of_openings_always_reads_back_the_same() {
    let mut s = Seed(0x5EED_0044);
    for round in 0..300 {
        let n = 1 + (s.next() % 12) as usize;
        let list: Vec<Opening> = (0..n)
            .map(|_| {
                let form = if s.next().is_multiple_of(2) {
                    Form::Round
                } else {
                    Form::Rectangle
                };
                let w = s.between(0.05, 5.0);
                let h = if form == Form::Round {
                    w
                } else {
                    s.between(0.05, 5.0)
                };
                open(
                    s.between(-200.0, 200.0),
                    s.between(-200.0, 200.0),
                    w,
                    h,
                    form,
                )
            })
            .collect();
        let (text, _) = to_dxf(&list, false).unwrap();
        let (back, notes) = from_dxf(&text).unwrap();
        assert!(notes.is_empty(), "round {round}: {notes:?}");
        assert_eq!(back.len(), list.len(), "round {round}");
        for (a, b) in list.iter().zip(&back) {
            assert!(same(a, b), "round {round}: {a:?} {b:?}");
        }
    }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("eh3d_stencil_dxfout_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_file_that_exists_is_never_replaced_by_write_new() {
    let dir = scratch("new");
    let file = dir.join("a.dxf");
    write_new(&file, "first").unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "first");
    let e = write_new(&file, "second").unwrap_err();
    assert!(
        e.starts_with("file_exists: ") && e.contains("--overwrite"),
        "{e}"
    );
    assert_eq!(fs::read_to_string(&file).unwrap(), "first");
    // a folder with that name, and a place that does not exist, are not written either
    let folder = dir.join("folder.dxf");
    fs::create_dir_all(&folder).unwrap();
    assert!(write_new(&folder, "x").is_err());
    assert!(folder.is_dir());
    let e = write_new(&dir.join("none").join("b.dxf"), "x").unwrap_err();
    assert!(e.starts_with("file_unwritable: "), "{e}");
    let _ = fs::remove_dir_all(&dir);
}

const PADS: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\n%ADD11C,0.8*%\nD10*\nX20000Y20000D03*\nD11*\nX80000Y20000D03*\nM02*\n";

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn convert_to_a_dxf_draws_the_reduced_openings_and_never_replaces_a_file_unless_asked() {
    let dir = scratch("cli");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, PADS).unwrap();
    let l = layer.to_str().unwrap();
    let out = dir.join("template.dxf");
    let o = out.to_str().unwrap();
    // 10 % less on each side of the pads, on a sheet of 0.1 mm
    let (code, text, why) = run(&["convert", l, o, "--thickness", "0.1", "--shrink", "10"]);
    assert_eq!(code, Some(0), "{why}");
    assert!(text.contains("2 openings drawn"), "{text}");
    let (back, notes) = from_dxf(&fs::read_to_string(&out).unwrap()).unwrap();
    assert!(notes.is_empty());
    assert_eq!(back.len(), 2);
    let want = [
        open(2.0, 2.0, 0.9, 0.45, Form::Rectangle),
        open(8.0, 2.0, 0.72, 0.72, Form::Round),
    ];
    for w in &want {
        assert!(back.iter().any(|b| same(w, b)), "{w:?} in {back:?}");
    }
    let first = fs::read(&out).unwrap();
    // the second time it is refused, with --create or without, and the file is as it was
    for extra in [vec![], vec!["--create"]] {
        let mut args = vec!["convert", l, o, "--thickness", "0.1", "--shrink", "20"];
        args.extend(extra);
        let (code, _, why) = run(&args);
        assert_eq!(code, Some(1), "{args:?}");
        assert!(why.contains("file_exists"), "{why}");
        assert_eq!(fs::read(&out).unwrap(), first);
    }
    // --overwrite replaces it, and no temporary file stays
    let (code, _, why) = run(&[
        "convert",
        l,
        o,
        "--thickness",
        "0.1",
        "--shrink",
        "20",
        "--overwrite",
    ]);
    assert_eq!(code, Some(0), "{why}");
    assert_ne!(fs::read(&out).unwrap(), first);
    assert!(!dir.join("template.dxf.tmp").exists());
    // the two together say opposite things; neither is an option of a G-code output or of check
    let (code, _, why) = run(&["convert", l, o, "--create", "--overwrite"]);
    assert_eq!(code, Some(1));
    assert!(why.contains("opposite"), "{why}");
    let nc = dir.join("a.nc");
    let (code, _, why) = run(&[
        "convert",
        l,
        nc.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--overwrite",
    ]);
    assert_eq!(code, Some(1));
    assert!(why.contains("of a DXF output"), "{why}");
    assert!(!nc.exists());
    let (code, _, why) = run(&["check", l, "--create"]);
    assert_eq!(code, Some(1));
    assert!(why.contains("options of convert"), "{why}");
    // the drawing is not the layer's own file and does not take the options of the cut
    let (code, _, why) = run(&[
        "convert",
        l,
        dir.join("o.dxf").to_str().unwrap(),
        "--origin",
        "0,0",
    ]);
    assert_eq!(code, Some(1));
    assert!(why.contains("not of a drawing"), "{why}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_obround_pad_stops_the_drawing_unless_force_leaves_it_out() {
    let dir = scratch("obround");
    let layer = dir.join("paste.gtp");
    fs::write(
        &layer,
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10O,1.0X0.5*%\n%ADD11C,0.8*%\nD10*\nX20000Y20000D03*\nD11*\nX80000Y20000D03*\nM02*\n",
    )
    .unwrap();
    let l = layer.to_str().unwrap();
    let out = dir.join("t.dxf");
    let o = out.to_str().unwrap();
    let (code, _, why) = run(&["convert", l, o, "--thickness", "0.1", "--shrink", "0"]);
    assert_eq!(code, Some(1), "{why}");
    assert!(why.contains("obround"), "{why}");
    assert!(!out.exists());
    let (code, text, why) = run(&[
        "convert",
        l,
        o,
        "--thickness",
        "0.1",
        "--shrink",
        "0",
        "--force",
    ]);
    assert_eq!(code, Some(0), "{why}");
    assert!(
        why.contains("1 obround opening(s) are not in the drawing"),
        "{why}"
    );
    assert!(text.contains("1 openings drawn"), "{text}");
    assert_eq!(
        from_dxf(&fs::read_to_string(&out).unwrap())
            .unwrap()
            .0
            .len(),
        1
    );
    let _ = fs::remove_dir_all(&dir);
}
