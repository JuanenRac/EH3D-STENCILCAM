//! The straight tangent lead-in and lead-out of every cut, and the settings file that gives their length: cases worked out by hand for a rectangle and a circle, the range of the
//! values, the walk of the strict mode over the result and the command line.

use eh3d_parser_gcode::{interpret, parse, Dialect, Motion, Severity};
use eh3d_stencilcam::config::{default_file, parse as parse_settings, parse_lead, Settings};
use eh3d_stencilcam::openings::{Form, Opening};
use eh3d_stencilcam::output::{gcode_with_leads, gcode_with_wait, lead_points, outline};
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

/// A rectangle 2 x 1 centred on the origin with sharp corners: (1, 0.5), (-1, 0.5), (-1, -0.5), (1, -0.5) and back to (1, 0.5).
fn rectangle() -> Vec<(f64, f64)> {
    outline(&opening(Form::Rectangle, 2.0, 1.0), 0.0)
}

/// A circle of radius 1 centred on the origin: it starts at (1, 0) and goes counter-clockwise in 32 chords of 11.25 degrees.
fn circle() -> Vec<(f64, f64)> {
    outline(&opening(Form::Round, 2.0, 2.0), 0.0)
}

#[test]
fn a_rectangle_enters_on_the_line_of_its_first_side_and_leaves_on_the_line_of_its_last() {
    // the first side runs from (1, 0.5) towards -x, so the lead-in starts 0.3 behind it at (1.3, 0.5); the last side runs up along x = 1 into (1, 0.5), so the lead-out
    // goes on to (1, 0.8). Both lie outside the rectangle, on the sheet.
    let g = gcode_with_leads(&[rectangle()], 600.0, 1000, 0.0, 0.3, 0.3).unwrap();
    assert_eq!(
        g,
        "G21\nG90\nF600\nG0 X1.3 Y0.5\nM3 S1000\nG1 X1 Y0.5\nG1 X-1 Y0.5\nG1 X-1 Y-0.5\nG1 X1 Y-0.5\nG1 X1 Y0.5\nG1 X1 Y0.8\nM5\nM2\n"
    );
}

#[test]
fn the_laser_is_switched_on_at_the_start_of_the_lead_in_and_waits_there() {
    // only a lead-in of 0.5 and a wait of 0.2 s: the wait comes right after M3, before the head moves, and there is no lead-out
    let g = gcode_with_leads(&[rectangle()], 600.0, 1000, 0.2, 0.5, 0.0).unwrap();
    assert_eq!(
        g,
        "G21\nG90\nF600\nG0 X1.5 Y0.5\nM3 S1000\nG4 P0.2\nG1 X1 Y0.5\nG1 X-1 Y0.5\nG1 X-1 Y-0.5\nG1 X1 Y-0.5\nG1 X1 Y0.5\nM5\nM2\n"
    );
    // only a lead-out of 0.25: the head goes to the first point as before
    let g = gcode_with_leads(&[rectangle()], 600.0, 1000, 0.0, 0.0, 0.25).unwrap();
    assert_eq!(
        g,
        "G21\nG90\nF600\nG0 X1 Y0.5\nM3 S1000\nG1 X-1 Y0.5\nG1 X-1 Y-0.5\nG1 X1 Y-0.5\nG1 X1 Y0.5\nG1 X1 Y0.75\nM5\nM2\n"
    );
}

#[test]
fn a_circle_has_its_leads_tangent_to_its_first_and_last_chords() {
    let c = circle();
    // the first chord goes from angle 0 to 11.25 degrees: its direction is (-sin 5.625, cos 5.625) = (-0.098017, 0.995185); 0.3 mm behind (1, 0) is (1.029405, -0.298555),
    // and the last chord comes up into (1, 0) with the direction (0.098017, 0.995185), so the lead-out ends at (1.029405, 0.298555)
    let (entry, exit) = lead_points(&c, 0.3, 0.3);
    let (entry, exit) = (entry.unwrap(), exit.unwrap());
    assert!(
        (entry.0 - 1.029_405).abs() < 1e-5 && (entry.1 + 0.298_555).abs() < 1e-5,
        "{entry:?}"
    );
    assert!(
        (exit.0 - 1.029_405).abs() < 1e-5 && (exit.1 - 0.298_555).abs() < 1e-5,
        "{exit:?}"
    );
    // both lie outside the circle, and exactly 0.3 mm from its start point
    for p in [entry, exit] {
        assert!(p.0.hypot(p.1) > 1.0, "{p:?}");
        assert!(((p.0 - 1.0).hypot(p.1) - 0.3).abs() < 1e-9, "{p:?}");
    }
    let g = gcode_with_leads(&[c], 600.0, 1000, 0.0, 0.3, 0.3).unwrap();
    assert!(
        g.contains("\nG0 X1.029 Y-0.299\nM3 S1000\nG1 X1 Y0\n"),
        "{g}"
    );
    assert!(g.ends_with("G1 X1.029 Y0.299\nM5\nM2\n"), "{g}");
}

#[test]
fn a_zero_lead_writes_the_program_it_wrote_before() {
    let paths = vec![
        rectangle(),
        circle(),
        outline(&opening(Form::Rectangle, 0.6, 0.4), 0.05),
    ];
    for wait in [0.0, 0.5] {
        assert_eq!(
            gcode_with_leads(&paths, 600.0, 1000, wait, 0.0, 0.0).unwrap(),
            gcode_with_wait(&paths, 600.0, 1000, wait).unwrap()
        );
    }
    // a lead is only added where its length is more than zero: three paths have three more lines with a lead-in alone
    let plain = gcode_with_wait(&paths, 600.0, 1000, 0.0).unwrap();
    let led = gcode_with_leads(&paths, 600.0, 1000, 0.0, 0.3, 0.0).unwrap();
    assert_eq!(led.lines().count(), plain.lines().count() + 3);
}

#[test]
fn a_path_that_starts_or_ends_with_repeated_points_still_has_a_direction() {
    // the first two points are the same and so are the last two: the direction comes from the first and the last segment that has a length
    let path = vec![
        (0.0, 0.0),
        (0.0, 0.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.0, 0.0),
        (0.0, 0.0),
    ];
    let (entry, exit) = lead_points(&path, 0.5, 0.5);
    let (entry, exit) = (entry.unwrap(), exit.unwrap());
    assert!(
        (entry.0 + 0.5).abs() < 1e-12 && entry.1.abs() < 1e-12,
        "{entry:?}"
    );
    // the last segment with a length goes from (1, 1) to (0, 0): the lead-out goes on along it
    let half_diagonal = -0.5 / 2.0_f64.sqrt();
    assert!(
        (exit.0 - half_diagonal).abs() < 1e-12 && (exit.1 - half_diagonal).abs() < 1e-12,
        "{exit:?}"
    );
    // a path of one point has no direction and no lead
    assert_eq!(
        lead_points(&[(1.0, 1.0), (1.0, 1.0)], 0.5, 0.5),
        (None, None)
    );
    assert_eq!(lead_points(&path, 0.0, 0.0), (None, None));
}

#[test]
fn a_lead_must_be_from_zero_to_five_millimetres() {
    for bad in [-0.1, 5.1, f64::NAN, f64::INFINITY] {
        let e = gcode_with_leads(&[rectangle()], 600.0, 1000, 0.0, bad, 0.0).unwrap_err();
        assert!(
            e.contains("lead-in") && e.contains("0 to 5 mm"),
            "{bad}: {e}"
        );
        let e = gcode_with_leads(&[rectangle()], 600.0, 1000, 0.0, 0.0, bad).unwrap_err();
        assert!(e.contains("lead-out"), "{bad}: {e}");
    }
    assert!(gcode_with_leads(&[rectangle()], 600.0, 1000, 0.0, 5.0, 5.0).is_ok());
}

/// What the strict mode of the G-code debugger would report for `text`.
fn alerts(text: &str) -> Vec<String> {
    let trace = interpret(&parse(text, Dialect::Grbl));
    let mut found: Vec<String> = trace
        .diagnostics
        .iter()
        .filter(|d| d.severity >= Severity::Warning)
        .map(|d| format!("line {}: {}", d.line, d.message))
        .collect();
    for s in &trace.segments {
        if !matches!(s.motion, Motion::Rapid) && !s.spindle_on {
            found.push(format!("line {}: it cuts with the laser off", s.line));
        }
    }
    found
}

#[test]
fn the_program_with_leads_never_cuts_with_the_laser_off() {
    let paths = vec![
        rectangle(),
        circle(),
        outline(&opening(Form::Rectangle, 0.6, 0.4), 0.05),
    ];
    let g = gcode_with_leads(&paths, 600.0, 1000, 0.1, 0.3, 0.3).unwrap();
    let found = alerts(&g);
    assert!(found.is_empty(), "{found:?}\n{g}");
    // the laser is on between each M3 and M5 and every travel with G0 is made with it off
    let mut on = false;
    for line in g.lines() {
        if line.starts_with("M3") {
            on = true;
        } else if line.starts_with("M5") {
            on = false;
        } else if line.starts_with("G0 ") {
            assert!(!on, "{g}");
        }
    }
}

#[test]
fn the_settings_file_has_the_leads_and_the_wait_with_comments_and_the_default() {
    assert_eq!(
        Settings::default(),
        Settings {
            lead_in_mm: 0.3,
            lead_out_mm: 0.3,
            laser_wait_s: 0.0,
            rapid_mm_min: 0.0
        }
    );
    assert_eq!(parse_settings("").unwrap(), Settings::default());
    assert_eq!(
        parse_settings("# a\n\n lead_in_mm = 0.5 # half\nlead_out_mm=0\nlaser_wait_s = 2.5\n")
            .unwrap(),
        Settings {
            lead_in_mm: 0.5,
            lead_out_mm: 0.0,
            laser_wait_s: 2.5,
            rapid_mm_min: 0.0
        }
    );
    assert_eq!(
        parse_settings(&default_file()).unwrap(),
        Settings::default()
    );
    let example =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/stencil.cfg")).unwrap();
    assert_eq!(example.replace("\r\n", "\n"), default_file());
    assert_eq!(parse_lead("--lead-in", " 1.5 ").unwrap(), 1.5);
}

#[test]
fn a_bad_settings_file_is_an_error_that_says_the_line() {
    for (text, line, word) in [
        ("lead_in_mm = 5.5\n", 1, "from 0 to 5 mm"),
        ("\nlead_out_mm = -1\n", 2, "from 0 to 5 mm"),
        ("laser_wait_s = 61\n", 1, "from 0 to 60 seconds"),
        ("lead_in_mm = long\n", 1, "'long'"),
        ("lead_in_mm = nan\n", 1, "'nan'"),
        ("# c\nlead_in_mm\n", 2, "key = value"),
        ("lead_in_mm = 0.3\nfeed = 1\n", 2, "not a setting"),
        ("lead_in = 0.3\n", 1, "not a setting"),
        ("lead_in_mm = 0.3\nlead_in_mm = 0.4\n", 2, "twice"),
        ("laser_wait_s = 1\nlaser_wait_s = 2\n", 2, "twice"),
    ] {
        let e = parse_settings(text).unwrap_err();
        assert!(
            e.starts_with(&format!("line {line}:")) && e.contains(word),
            "{text:?}: {e}"
        );
    }
}

const TWO_PADS: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn convert(dir: &std::path::Path, name: &str, extra: &[&str]) -> Result<String, String> {
    let layer = dir.join("paste.gtp");
    let out = dir.join(name);
    let mut args = vec![
        "convert",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--thickness",
        "0.1",
    ];
    args.extend_from_slice(extra);
    let (ok, _, e) = run(&args);
    if ok {
        Ok(fs::read_to_string(&out).unwrap())
    } else {
        Err(e)
    }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("eh3d_stencil_leads_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("paste.gtp"), TWO_PADS).unwrap();
    dir
}

fn moves(g: &str) -> usize {
    g.lines().filter(|l| l.starts_with("G1 X")).count()
}

#[test]
fn the_command_line_adds_leads_by_default_and_zero_gives_the_old_program() {
    let dir = scratch("default");
    let led = convert(&dir, "led.nc", &[]).unwrap();
    let none = convert(&dir, "none.nc", &["--lead-in", "0", "--lead-out", "0"]).unwrap();
    // two openings, a lead-in and a lead-out each
    assert_eq!(moves(&led), moves(&none) + 4, "{led}");
    // a lead-in alone, a lead-out alone
    let only_in = convert(&dir, "in.nc", &["--lead-out", "0"]).unwrap();
    let only_out = convert(&dir, "out.nc", &["--lead-in", "0"]).unwrap();
    assert_eq!(moves(&only_in), moves(&none) + 2);
    assert_eq!(moves(&only_out), moves(&none) + 2);
    // the plotter file is not touched by the leads
    let plt = convert(&dir, "a.plt", &[]).unwrap();
    let plt0 = convert(&dir, "b.plt", &["--lead-in", "0", "--lead-out", "0"]).unwrap();
    assert_eq!(plt, plt0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_settings_file_of_the_command_line_is_read_and_the_options_win() {
    let dir = scratch("config");
    let cfg = dir.join("stencil.cfg");
    let c = cfg.to_str().unwrap();
    fs::write(
        &cfg,
        "# no leads, a wait\nlead_in_mm = 0\nlead_out_mm = 0\nlaser_wait_s = 1\n",
    )
    .unwrap();
    let none = convert(&dir, "none.nc", &["--lead-in", "0", "--lead-out", "0"]).unwrap();
    let from_file = convert(&dir, "file.nc", &["--config", c]).unwrap();
    assert_eq!(moves(&from_file), moves(&none));
    assert_eq!(from_file.matches("G4 P1\n").count(), 2, "{from_file}");
    // the options win over the file
    let won = convert(
        &dir,
        "won.nc",
        &["--config", c, "--lead-in", "0.4", "--laser-wait", "0"],
    )
    .unwrap();
    assert_eq!(moves(&won), moves(&none) + 2);
    assert!(!won.contains("G4"), "{won}");
    // a bad file is an error with its name and the number of the line, and nothing is written
    fs::write(&cfg, "lead_in_mm = 0.3\nlead_out_mm = 9\n").unwrap();
    let e = convert(&dir, "bad.nc", &["--config", c]).unwrap_err();
    assert!(e.contains("stencil.cfg") && e.contains("line 2"), "{e}");
    assert!(!dir.join("bad.nc").exists());
    let e = convert(&dir, "missing.nc", &["--config", "no-such-file.cfg"]).unwrap_err();
    assert!(e.contains("no-such-file.cfg"), "{e}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_lead_options_are_checked_and_the_config_command_prints_the_default_file() {
    let dir = scratch("bad");
    for (flag, bad) in [("--lead-in", "6"), ("--lead-out", "-1"), ("--lead-in", "x")] {
        let e = convert(&dir, "o.nc", &[flag, bad]).unwrap_err();
        assert!(
            e.contains(flag) && e.contains("0 to 5 mm"),
            "{flag} {bad}: {e}"
        );
    }
    assert!(!dir.join("o.nc").exists());
    let (ok, text, e) = run(&["config"]);
    assert!(ok, "{e}");
    assert_eq!(text, default_file());
    let (ok, _, e) = run(&["config", "extra"]);
    assert!(!ok && e.contains("extra"), "{e}");
    let _ = fs::remove_dir_all(&dir);
}
