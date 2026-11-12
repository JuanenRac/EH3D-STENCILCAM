//! The drawing of a stencil in a DXF file, and the speed, the laser scale and the passes of the cut: what becomes an opening and what is a note, the unit of the drawing, the settings
//! file and the command line over the example files.

use eh3d_stencilcam::config::{
    default_file, parse as parse_settings, parse_feed, parse_passes, parse_smax, Settings,
};
use eh3d_stencilcam::dxf_in::{from_dxf, is_drawing};
use eh3d_stencilcam::openings::Form;
use std::fs;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-stencilcam");

fn root(name: &str) -> String {
    format!("{}/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn drawing(units: Option<i64>, entities: &str) -> String {
    let header = units.map_or(String::new(), |u| {
        format!("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n{u}\n0\nENDSEC\n")
    });
    format!("{header}0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n")
}

fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> String {
    let mut s = String::from("0\nLWPOLYLINE\n8\n0\n90\n4\n70\n1\n");
    for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
        s += &format!("10\n{x}\n20\n{y}\n");
    }
    s
}

#[test]
fn a_circle_is_a_round_opening_and_a_closed_rectangle_a_rectangular_one() {
    let text = drawing(
        Some(4),
        &(rectangle(1.5, 1.75, 2.5, 2.25) + "0\nCIRCLE\n8\n0\n10\n6\n20\n2\n40\n0.3\n"),
    );
    let (openings, notes) = from_dxf(&text).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(openings.len(), 2);
    let r = &openings[0];
    assert_eq!(
        (r.form, r.x, r.y, r.w, r.h),
        (Form::Rectangle, 2.0, 2.0, 1.0, 0.5)
    );
    let c = &openings[1];
    assert_eq!((c.form, c.x, c.y), (Form::Round, 6.0, 2.0));
    assert!((c.w - 0.6).abs() < 1e-12 && (c.h - 0.6).abs() < 1e-12);
}

#[test]
fn the_unit_of_the_drawing_is_turned_into_millimetres() {
    let inches = drawing(Some(1), "0\nCIRCLE\n8\n0\n10\n1\n20\n2\n40\n0.5\n");
    let (o, _) = from_dxf(&inches).unwrap();
    assert!(
        (o[0].x - 25.4).abs() < 1e-9
            && (o[0].y - 50.8).abs() < 1e-9
            && (o[0].w - 25.4).abs() < 1e-9
    );
    // a drawing that does not say is in millimetres
    let (o, _) = from_dxf(&drawing(None, "0\nCIRCLE\n8\n0\n10\n1\n20\n2\n40\n0.5\n")).unwrap();
    assert_eq!((o[0].x, o[0].w), (1.0, 1.0));
}

#[test]
fn what_cannot_be_an_opening_is_a_note_once_for_each_kind() {
    let mut entities = String::new();
    entities += "0\nLINE\n8\n0\n10\n0\n20\n0\n11\n5\n21\n0\n";
    entities += "0\nLINE\n8\n0\n10\n0\n20\n1\n11\n5\n21\n1\n";
    entities += "0\nARC\n8\n0\n10\n0\n20\n0\n40\n1\n50\n0\n51\n90\n";
    // a square turned 45 degrees and a triangle: polylines that are not rectangles along the axes
    entities += "0\nLWPOLYLINE\n8\n0\n90\n4\n70\n1\n10\n1\n20\n0\n10\n2\n20\n1\n10\n1\n20\n2\n10\n0\n20\n1\n";
    entities += "0\nLWPOLYLINE\n8\n0\n90\n3\n70\n1\n10\n0\n20\n0\n10\n1\n20\n0\n10\n0\n20\n1\n";
    entities += "0\nCIRCLE\n8\n0\n10\n3\n20\n3\n40\n0.4\n";
    let (openings, notes) = from_dxf(&drawing(Some(4), &entities)).unwrap();
    assert_eq!(openings.len(), 1);
    assert_eq!(notes.len(), 3, "{notes:?}"); // LINE once, ARC once, LWPOLYLINE once
    for kind in ["LINE", "ARC", "LWPOLYLINE"] {
        assert!(notes.iter().any(|n| n.contains(kind)), "{kind}: {notes:?}");
    }
    // an open polyline and a polyline with a bulge are not rectangles either
    let open = drawing(
        Some(4),
        &rectangle(0.0, 0.0, 1.0, 1.0).replace("70\n1\n", "70\n0\n"),
    );
    assert!(from_dxf(&open).unwrap().0.is_empty());
    let bulged = drawing(
        Some(4),
        &rectangle(0.0, 0.0, 1.0, 1.0).replacen("20\n0\n", "20\n0\n42\n0.5\n", 1),
    );
    assert!(from_dxf(&bulged).unwrap().0.is_empty());
}

#[test]
fn a_drawing_that_is_not_one_or_has_nothing_to_cut_is_said() {
    assert!(from_dxf("").unwrap_err().contains("empty"));
    assert!(from_dxf("not a group code\nvalue\n")
        .unwrap_err()
        .contains("group code"));
    let (o, n) = from_dxf(&drawing(Some(4), "")).unwrap();
    assert!(o.is_empty() && n.len() == 1 && n[0].contains("nothing to cut"));
    // a circle with no radius is not an opening
    let (o, n) = from_dxf(&drawing(Some(4), "0\nCIRCLE\n8\n0\n10\n0\n20\n0\n40\n0\n")).unwrap();
    assert!(o.is_empty() && !n.is_empty());
}

#[test]
fn a_file_is_a_drawing_by_its_extension_in_either_case() {
    assert!(is_drawing("paste.dxf") && is_drawing("PASTE.DXF") && is_drawing("a/b.c/paste.Dxf"));
    assert!(!is_drawing("paste.gtp") && !is_drawing("dxf") && !is_drawing("paste.dxf.nc"));
}

#[test]
fn the_speed_the_scale_and_the_passes_have_their_defaults_and_their_ranges() {
    let d = Settings::default();
    assert_eq!((d.feed_mm_min, d.smax, d.passes), (600.0, 1000.0, 1));
    let s = parse_settings("feed_mm_min = 300\nsmax = 255\npasses = 3\n").unwrap();
    assert_eq!((s.feed_mm_min, s.smax, s.passes), (300.0, 255.0, 3));
    assert_eq!(parse_settings(&default_file()).unwrap(), d);
    assert!(
        parse_feed("--feed", "0.5").is_err()
            && parse_feed("--feed", "100001").is_err()
            && parse_feed("--feed", "nan").is_err()
            && parse_feed("--feed", "1").is_ok()
    );
    assert!(
        parse_smax("--smax", "0").is_err()
            && parse_smax("--smax", "100001").is_err()
            && parse_smax("--smax", "100000").is_ok()
    );
    assert!(
        parse_passes("--passes", "0").is_err()
            && parse_passes("--passes", "11").is_err()
            && parse_passes("--passes", "1.5").is_err()
            && parse_passes("--passes", "10").is_ok()
    );
    let e = parse_settings("lead_in_mm = 0.3\npasses = 0\n").unwrap_err();
    assert!(e.starts_with("line 2:") && e.contains("passes"), "{e}");
    assert!(parse_settings("passes = 2\npasses = 3\n")
        .unwrap_err()
        .contains("twice"));
    let example = fs::read_to_string(root("examples/stencil.cfg")).unwrap();
    assert_eq!(example.replace("\r\n", "\n"), default_file());
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn convert_reads_a_drawing_and_cuts_every_opening_of_it() {
    let out = format!("{}/stencil_from_dxf.nc", env!("CARGO_TARGET_TMPDIR"));
    let _ = fs::remove_file(&out);
    let (ok, o, e) = run(&["convert", &root("examples/paste.dxf"), &out, "--force"]);
    assert!(ok, "{e}");
    assert!(o.contains("3 openings written"), "{o}");
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(text.matches("M3 S1000").count(), 3);
    assert!(text.contains("source") || text.contains("paste.dxf"));
}

#[test]
fn the_passes_cut_all_the_contours_again_and_the_feed_and_the_scale_come_from_the_options() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let (one, three) = (
        format!("{dir}/stencil_one_pass.nc"),
        format!("{dir}/stencil_three_passes.nc"),
    );
    let _ = fs::remove_file(&one);
    let _ = fs::remove_file(&three);
    let (ok, _, e) = run(&["convert", &root("examples/paste.dxf"), &one, "--force"]);
    assert!(ok, "{e}");
    let (ok, o, e) = run(&[
        "convert",
        &root("examples/paste.dxf"),
        &three,
        "--passes",
        "3",
        "--feed",
        "250",
        "--smax",
        "255",
        "--force",
    ]);
    assert!(ok, "{e}");
    assert!(
        o.contains("3 openings written") && o.contains("3 passes"),
        "{o}"
    );
    let (a, b) = (
        fs::read_to_string(&one).unwrap(),
        fs::read_to_string(&three).unwrap(),
    );
    assert_eq!(a.matches("M3 S").count(), 3);
    assert_eq!(b.matches("M3 S255").count(), 9);
    assert!(b.contains("F250") && !b.contains("F600"));
}

#[test]
fn a_value_out_of_range_is_refused_with_the_name_of_the_option() {
    let out = format!("{}/stencil_refused.nc", env!("CARGO_TARGET_TMPDIR"));
    for (flag, value) in [
        ("--passes", "0"),
        ("--passes", "11"),
        ("--feed", "0"),
        ("--smax", "0"),
    ] {
        let (ok, _, e) = run(&["convert", &root("examples/paste.dxf"), &out, flag, value]);
        assert!(!ok && e.contains(flag), "{flag} {value}: {e}");
    }
}
