//! The lead-in and the lead-out by form: a round opening and a rectangular one can have their own, and the ones that are not given are the general ones. Values worked out by hand for
//! a rectangle of 2 x 1 mm (its first side runs from (1, 0.5) towards -x and its last one comes up into (1, 0.5)) and a circle (its first chord leaves (1, 0) in the direction
//! (-sin 5.625 degrees, cos 5.625 degrees) = (-0.098017, 0.995185)).

use eh3d_stencilcam::config::{default_file, parse, Settings};
use eh3d_stencilcam::openings::{Form, Opening};
use eh3d_stencilcam::output::{
    estimate_time, estimate_time_with_forms, forms, gcode_with_form_leads, gcode_with_leads,
    ordered_indices, outline, FormLeads, Lead,
};
use std::fs;
use std::process::Command;

fn opening(form: Form, w: f64, h: f64) -> Opening {
    Opening {
        x: 0.0,
        y: 0.0,
        w,
        h,
        form,
    }
}

/// A rectangle 2 x 1 centred on the origin with sharp corners.
fn rectangle() -> Vec<(f64, f64)> {
    outline(&opening(Form::Rectangle, 2.0, 1.0), 0.0)
}

/// The same rectangle 10 mm to the right.
fn far_rectangle() -> Vec<(f64, f64)> {
    rectangle()
        .into_iter()
        .map(|(x, y)| (x + 10.0, y))
        .collect()
}

/// General leads of 0.3, round ones of 0.2 and 0.4, rectangular ones of 0.5 and 0.1.
fn leads() -> FormLeads {
    FormLeads {
        general: Lead {
            in_mm: 0.3,
            out_mm: 0.3,
        },
        round: Lead {
            in_mm: 0.2,
            out_mm: 0.4,
        },
        rect: Lead {
            in_mm: 0.5,
            out_mm: 0.1,
        },
    }
}

const HEAD: &str = "G21\nG90\nF600\n";
/// The cut of the near rectangle with a round lead-in of 0.2 and a round lead-out of 0.4.
const NEAR_ROUND: &str = "G0 X1.2 Y0.5\nM3 S1000\nG1 X1 Y0.5\nG1 X-1 Y0.5\nG1 X-1 Y-0.5\nG1 X1 Y-0.5\nG1 X1 Y0.5\nG1 X1 Y0.9\nM5\n";
/// The cut of the far rectangle with a rectangular lead-in of 0.5 and lead-out of 0.1.
const FAR_RECT: &str = "G0 X11.5 Y0.5\nM3 S1000\nG1 X11 Y0.5\nG1 X9 Y0.5\nG1 X9 Y-0.5\nG1 X11 Y-0.5\nG1 X11 Y0.5\nG1 X11 Y0.6\nM5\n";

#[test]
fn each_path_gets_the_leads_of_its_form() {
    let g = gcode_with_form_leads(
        &[rectangle(), far_rectangle()],
        &[Form::Round, Form::Rectangle],
        600.0,
        1000,
        0.0,
        &leads(),
    )
    .unwrap();
    assert_eq!(g, format!("{HEAD}{NEAR_ROUND}{FAR_RECT}M2\n"));
}

#[test]
fn the_form_follows_its_path_when_the_walk_changes_the_order() {
    // the far one is first in the list but the walk starts at the origin: the near one is cut first, and the forms stay with their paths
    let paths = [far_rectangle(), rectangle()];
    assert_eq!(ordered_indices(&paths), vec![1, 0]);
    let g = gcode_with_form_leads(
        &paths,
        &[Form::Rectangle, Form::Round],
        600.0,
        1000,
        0.0,
        &leads(),
    )
    .unwrap();
    assert_eq!(g, format!("{HEAD}{NEAR_ROUND}{FAR_RECT}M2\n"));
    assert!(ordered_indices(&[]).is_empty());
}

#[test]
fn an_obround_opening_uses_the_general_leads() {
    let g = gcode_with_form_leads(&[rectangle()], &[Form::Obround], 600.0, 1000, 0.0, &leads())
        .unwrap();
    assert_eq!(
        g,
        gcode_with_leads(&[rectangle()], 600.0, 1000, 0.0, 0.3, 0.3).unwrap()
    );
    assert!(
        g.contains("G0 X1.3 Y0.5\n") && g.contains("G1 X1 Y0.8\nM5\n"),
        "{g}"
    );
}

#[test]
fn the_same_leads_for_every_form_write_the_program_of_before() {
    let paths = [rectangle(), far_rectangle()];
    let same = FormLeads::uniform(0.3, 0.25);
    for f in [Form::Round, Form::Rectangle, Form::Obround] {
        assert_eq!(
            same.for_form(f),
            Lead {
                in_mm: 0.3,
                out_mm: 0.25
            }
        );
    }
    assert_eq!(
        gcode_with_form_leads(
            &paths,
            &[Form::Round, Form::Rectangle],
            600.0,
            1000,
            0.2,
            &same
        )
        .unwrap(),
        gcode_with_leads(&paths, 600.0, 1000, 0.2, 0.3, 0.25).unwrap()
    );
}

#[test]
fn a_circle_has_its_round_leads_tangent_to_its_chords() {
    // a lead-in of 0.2 behind (1, 0): (1 + 0.2 * 0.098017, -0.2 * 0.995185) = (1.019603, -0.199037); a lead-out of 0.4 beyond it: (1.039207, 0.398074)
    let c = outline(&opening(Form::Round, 2.0, 2.0), 0.0);
    let g = gcode_with_form_leads(&[c], &[Form::Round], 600.0, 1000, 0.0, &leads()).unwrap();
    assert!(
        g.contains("\nG0 X1.02 Y-0.199\nM3 S1000\nG1 X1 Y0\n"),
        "{g}"
    );
    assert!(g.ends_with("G1 X1.039 Y0.398\nM5\nM2\n"), "{g}");
}

#[test]
fn the_forms_must_be_as_many_as_the_paths() {
    let e = gcode_with_form_leads(&[rectangle()], &[], 600.0, 1000, 0.0, &leads()).unwrap_err();
    assert!(e.contains("0 forms for 1 paths"), "{e}");
    let e = estimate_time_with_forms(
        &[rectangle()],
        &[Form::Round; 2],
        600.0,
        0.0,
        &leads(),
        3000.0,
    )
    .unwrap_err();
    assert!(e.contains("2 forms for 1 paths"), "{e}");
}

#[test]
fn every_length_is_checked_with_its_own_name() {
    for (name, set) in [
        ("lead-in-round", 0usize),
        ("lead-out-round", 1),
        ("lead-in-rect", 2),
        ("lead-out-rect", 3),
    ] {
        for bad in [-0.1, 5.1, f64::NAN] {
            let mut l = leads();
            match set {
                0 => l.round.in_mm = bad,
                1 => l.round.out_mm = bad,
                2 => l.rect.in_mm = bad,
                _ => l.rect.out_mm = bad,
            }
            let e = gcode_with_form_leads(&[rectangle()], &[Form::Round], 600.0, 1000, 0.0, &l)
                .unwrap_err();
            assert!(
                e.contains(&format!("the {name} is")) && e.contains("0 to 5 mm"),
                "{name} {bad}: {e}"
            );
        }
    }
    assert!(gcode_with_form_leads(
        &[rectangle()],
        &[Form::Rectangle],
        600.0,
        1000,
        0.0,
        &FormLeads::uniform(5.0, 5.0)
    )
    .is_ok());
}

#[test]
fn the_forms_of_the_openings_are_listed_in_order() {
    let list = [
        opening(Form::Round, 1.0, 1.0),
        opening(Form::Rectangle, 2.0, 1.0),
        opening(Form::Obround, 2.0, 1.0),
    ];
    assert_eq!(
        forms(&list),
        vec![Form::Round, Form::Rectangle, Form::Obround]
    );
    assert!(forms(&[]).is_empty());
}

#[test]
fn the_time_counts_the_leads_of_each_form() {
    // cutting: the near rectangle 0.2 + 6 + 0.4 = 6.6 mm and the far one 0.5 + 6 + 0.1 = 6.6 mm, 13.2 mm at 600 mm/min = 1.32 s; waiting: 2 s twice = 4 s; travelling at
    // 3000 mm/min: the origin to (1.2, 0.5) is 1.3 mm and the end of the first lead-out (1, 0.9) to (11.5, 0.5) is hypot(10.5, 0.4) = 10.507616 mm, 11.807616 mm = 0.236152 s
    let t = estimate_time_with_forms(
        &[rectangle(), far_rectangle()],
        &[Form::Round, Form::Rectangle],
        600.0,
        2.0,
        &leads(),
        3000.0,
    )
    .unwrap();
    assert!((t.cutting_s - 1.32).abs() < 1e-9, "{t:?}");
    assert!((t.waiting_s - 4.0).abs() < 1e-12, "{t:?}");
    assert!((t.rapid_s - 0.236_152_325_7).abs() < 1e-9, "{t:?}");
    // the same leads for every form give the estimate of before
    let paths = [rectangle(), far_rectangle()];
    let before = estimate_time(&paths, 600.0, 2.0, (0.3, 0.25), 3000.0).unwrap();
    let after = estimate_time_with_forms(
        &paths,
        &[Form::Round, Form::Rectangle],
        600.0,
        2.0,
        &FormLeads::uniform(0.3, 0.25),
        3000.0,
    )
    .unwrap();
    assert_eq!(before, after);
}

#[test]
fn the_settings_file_has_the_leads_of_each_form_and_they_fall_back_to_the_general_ones() {
    let d = Settings::default();
    assert_eq!(
        (
            d.lead_in_round_mm,
            d.lead_out_round_mm,
            d.lead_in_rect_mm,
            d.lead_out_rect_mm
        ),
        (None, None, None, None)
    );
    assert_eq!(d.leads(), FormLeads::uniform(0.3, 0.3));
    let s = parse("lead_in_mm = 0.4\nlead_in_round_mm = 0.2 # a circle\nlead_out_rect_mm = 0\n")
        .unwrap();
    assert_eq!(s.lead_in_round_mm, Some(0.2));
    assert_eq!(s.lead_out_rect_mm, Some(0.0));
    assert_eq!((s.lead_out_round_mm, s.lead_in_rect_mm), (None, None));
    assert_eq!(
        s.leads(),
        FormLeads {
            general: Lead {
                in_mm: 0.4,
                out_mm: 0.3
            },
            round: Lead {
                in_mm: 0.2,
                out_mm: 0.3
            },
            rect: Lead {
                in_mm: 0.4,
                out_mm: 0.0
            },
        }
    );
    // the default file shows the four keys as comments and reads back as the defaults
    let text = default_file();
    for key in [
        "lead_in_round_mm",
        "lead_out_round_mm",
        "lead_in_rect_mm",
        "lead_out_rect_mm",
    ] {
        assert!(text.contains(&format!("\n# {key} = 0.3\n")), "{key}");
    }
    assert_eq!(parse(&text).unwrap(), Settings::default());
    for (text, line, word) in [
        ("lead_in_round_mm = 5.5\n", 1, "from 0 to 5 mm"),
        ("\nlead_out_round_mm = -1\n", 2, "from 0 to 5 mm"),
        ("lead_in_rect_mm = wide\n", 1, "'wide'"),
        ("lead_out_rect_mm = nan\n", 1, "'nan'"),
        ("lead_in_rect_mm = 1\nlead_in_rect_mm = 2\n", 2, "twice"),
        (
            "lead_out_round_mm = 1\n\nlead_out_round_mm = 2\n",
            3,
            "twice",
        ),
        ("lead_round = 1\n", 1, "not a setting"),
    ] {
        let e = parse(text).unwrap_err();
        assert!(
            e.starts_with(&format!("line {line}:")) && e.contains(word),
            "{text:?}: {e}"
        );
    }
}

#[test]
fn the_example_file_is_the_default_file() {
    let example =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/stencil.cfg")).unwrap();
    assert_eq!(example.replace("\r\n", "\n"), default_file());
}

/// A rectangle of 1 x 0.5 mm centred at (2, 2) and a circle of 1 mm across centred at (8, 2).
const PADS: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\n%ADD11C,1.0*%\nD10*\nX20000Y20000D03*\nD11*\nX80000Y20000D03*\nM02*\n";

fn run(args: &[&str]) -> (bool, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// Converts the pads with no shrink, no kerf and sharp corners, so that the lines of the cut are the pads.
fn convert(dir: &std::path::Path, name: &str, extra: &[&str]) -> Result<String, String> {
    let layer = dir.join("paste.gtp");
    let out = dir.join(name);
    let mut args = vec![
        "convert",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--shrink",
        "0",
        "--kerf",
        "0",
        "--radius",
        "0",
    ];
    args.extend_from_slice(extra);
    let (ok, e) = run(&args);
    if ok {
        Ok(fs::read_to_string(&out).unwrap())
    } else {
        Err(e)
    }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("eh3d_stencil_forms_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("paste.gtp"), PADS).unwrap();
    dir
}

// the rectangle at (2, 2) starts at (2.5, 2.25), its first side runs towards -x and its last one comes up into the start
const RECT_CUT: &str =
    "G1 X2.5 Y2.25\nG1 X1.5 Y2.25\nG1 X1.5 Y1.75\nG1 X2.5 Y1.75\nG1 X2.5 Y2.25\n";
// with no lead-in the head is already at the first point, and the cut starts with the second one
const RECT_REST: &str = "G1 X1.5 Y2.25\nG1 X1.5 Y1.75\nG1 X2.5 Y1.75\nG1 X2.5 Y2.25\n";

#[test]
fn the_command_line_gives_each_form_its_leads() {
    let dir = scratch("options");
    // the rectangle 0.5 in and 0.1 out: (3, 2.25) and (2.5, 2.35); the circle 0.2 in and 0.45 out behind and beyond (8.5, 2): (8.520, 1.801) and (8.544, 2.448)
    let g = convert(
        &dir,
        "both.nc",
        &[
            "--lead-in-rect",
            "0.5",
            "--lead-out-rect",
            "0.1",
            "--lead-in-round",
            "0.2",
            "--lead-out-round",
            "0.45",
        ],
    )
    .unwrap();
    assert!(
        g.contains(&format!(
            "G0 X3 Y2.25\nM3 S1000\n{RECT_CUT}G1 X2.5 Y2.35\nM5\n"
        )),
        "{g}"
    );
    assert!(g.contains("G0 X8.52 Y1.801\nM3 S1000\nG1 X8.5 Y2\n"), "{g}");
    assert!(g.contains("G1 X8.544 Y2.448\nM5\nM2\n"), "{g}");
    // the round ones alone: the rectangle keeps the general 0.3
    let g = convert(
        &dir,
        "round.nc",
        &["--lead-in-round", "0.2", "--lead-out-round", "0.45"],
    )
    .unwrap();
    assert!(
        g.contains(&format!(
            "G0 X2.8 Y2.25\nM3 S1000\n{RECT_CUT}G1 X2.5 Y2.55\nM5\n"
        )),
        "{g}"
    );
    assert!(g.contains("G0 X8.52 Y1.801\n"), "{g}");
    // the general options reach a form that has none of its own
    let g = convert(
        &dir,
        "general.nc",
        &["--lead-in", "0.2", "--lead-out", "0.45"],
    )
    .unwrap();
    assert!(
        g.contains(&format!(
            "G0 X2.7 Y2.25\nM3 S1000\n{RECT_CUT}G1 X2.5 Y2.7\nM5\n"
        )),
        "{g}"
    );
    assert!(
        g.contains("G0 X8.52 Y1.801\n") && g.contains("G1 X8.544 Y2.448\n"),
        "{g}"
    );
    // no lead for the rectangle: it starts at its first point and has no lead-out
    let g = convert(
        &dir,
        "norect.nc",
        &["--lead-in-rect", "0", "--lead-out-rect", "0"],
    )
    .unwrap();
    assert!(
        g.contains(&format!("G0 X2.5 Y2.25\nM3 S1000\n{RECT_REST}M5\n")),
        "{g}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_settings_file_gives_the_leads_of_the_forms_and_the_options_win() {
    let dir = scratch("config");
    let cfg = dir.join("stencil.cfg");
    let c = cfg.to_str().unwrap();
    fs::write(&cfg, "lead_in_rect_mm = 0.5\nlead_out_rect_mm = 0.1\n").unwrap();
    let g = convert(&dir, "file.nc", &["--config", c]).unwrap();
    assert!(
        g.contains(&format!(
            "G0 X3 Y2.25\nM3 S1000\n{RECT_CUT}G1 X2.5 Y2.35\nM5\n"
        )),
        "{g}"
    );
    // the circle has none of its own: the general 0.3
    assert!(g.contains("G0 X8.529 Y1.701\n"), "{g}");
    // the option wins over the key of the file; the general option does not override a key of its own
    let g = convert(
        &dir,
        "won.nc",
        &["--config", c, "--lead-in-rect", "0", "--lead-in", "0.2"],
    )
    .unwrap();
    assert!(
        g.contains(&format!(
            "G0 X2.5 Y2.25\nM3 S1000\n{RECT_REST}G1 X2.5 Y2.35\nM5\n"
        )),
        "{g}"
    );
    // a bad key of the file is an error with its line and nothing is written
    fs::write(&cfg, "lead_in_mm = 0.3\nlead_out_rect_mm = 9\n").unwrap();
    let e = convert(&dir, "bad.nc", &["--config", c]).unwrap_err();
    assert!(
        e.contains("stencil.cfg") && e.contains("line 2") && e.contains("lead_out_rect_mm"),
        "{e}"
    );
    assert!(!dir.join("bad.nc").exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_options_of_the_forms_are_checked_and_the_plotter_file_ignores_them() {
    let dir = scratch("bad");
    for (flag, bad) in [
        ("--lead-in-round", "6"),
        ("--lead-out-round", "-1"),
        ("--lead-in-rect", "x"),
        ("--lead-out-rect", "5.1"),
    ] {
        let e = convert(&dir, "o.nc", &[flag, bad]).unwrap_err();
        assert!(
            e.contains(flag) && e.contains("0 to 5 mm"),
            "{flag} {bad}: {e}"
        );
    }
    assert!(!dir.join("o.nc").exists());
    let e = convert(&dir, "o.nc", &["--lead-in-round"]).unwrap_err();
    assert!(e.contains("needs a value"), "{e}");
    let a = convert(&dir, "a.plt", &[]).unwrap();
    let b = convert(
        &dir,
        "b.plt",
        &["--lead-in-round", "1", "--lead-out-rect", "2"],
    )
    .unwrap();
    assert_eq!(a, b);
    let _ = fs::remove_dir_all(&dir);
}
