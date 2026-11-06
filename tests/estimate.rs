//! The estimated time of a job: values worked out by hand for a rectangle (and for two), the rapid speed of the settings file and of the command line, and the text of a duration.

use eh3d_stencilcam::config::{parse as parse_settings, parse_rapid};
use eh3d_stencilcam::openings::{Form, Opening};
use eh3d_stencilcam::output::{duration_text, estimate_time, outline};
use std::fs;
use std::process::Command;

fn rectangle_at(x: f64) -> Vec<(f64, f64)> {
    // 2 x 1 with sharp corners: starts at (x + 1, 0.5), goes round counter-clockwise and ends where it started; the perimeter is 6
    outline(
        &Opening {
            x,
            y: 0.0,
            w: 2.0,
            h: 1.0,
            form: Form::Rectangle,
        },
        0.0,
    )
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn a_rectangle_with_leads_takes_the_cut_the_wait_and_the_walk_from_the_origin() {
    // cut: lead-in 0.3 + perimeter 6 + lead-out 0.3 = 6.6 mm at 600 mm/min = 0.66 s
    // wait: one M3, 2 s
    // walk: from (0, 0) to the start of the lead-in (1.3, 0.5): sqrt(1.69 + 0.25) = 1.3928388277 mm at 3000 mm/min = 0.0278567766 s
    let t = estimate_time(&[rectangle_at(0.0)], 600.0, 2.0, (0.3, 0.3), 3000.0).unwrap();
    assert!(close(t.cutting_s, 0.66), "{t:?}");
    assert!(close(t.waiting_s, 2.0), "{t:?}");
    assert!(close(t.rapid_s, 0.027_856_776_554_368_24), "{t:?}");
    assert!(close(t.total_s(), 2.687_856_776_554_368), "{t:?}");
}

#[test]
fn without_leads_and_without_a_wait_only_the_perimeter_and_the_walk_count() {
    // cut: 6 mm at 600 mm/min = 0.6 s; walk: (0, 0) to (1, 0.5) = 1.1180339887 mm at 3000 mm/min = 0.0223606798 s
    let t = estimate_time(&[rectangle_at(0.0)], 600.0, 0.0, (0.0, 0.0), 3000.0).unwrap();
    assert!(close(t.cutting_s, 0.6), "{t:?}");
    assert_eq!(t.waiting_s, 0.0);
    assert!(close(t.rapid_s, 0.022_360_679_774_997_9), "{t:?}");
}

#[test]
fn two_rectangles_are_walked_from_the_end_of_the_lead_out_of_the_first() {
    // the second rectangle is 10 mm to the right. The first one ends at (1, 0.5) and its lead-out at (1, 0.8); the second lead-in starts at (11.3, 0.5).
    // walk: 1.3928388277 + sqrt(10.3^2 + 0.3^2) = 1.3928388277 + 10.3043680059 = 11.6972068336 mm at 3000 mm/min = 0.2339441367 s
    // cut: two times 6.6 mm = 13.2 mm at 600 mm/min = 1.32 s; wait: two times 2 s
    // the list is given far one first: the walk goes to the nearest start from the origin anyway
    let both = [rectangle_at(10.0), rectangle_at(0.0)];
    let t = estimate_time(&both, 600.0, 2.0, (0.3, 0.3), 3000.0).unwrap();
    assert!(close(t.cutting_s, 1.32), "{t:?}");
    assert!(close(t.waiting_s, 4.0), "{t:?}");
    assert!(close(t.rapid_s, 0.233_944_136_671_382_7), "{t:?}");
}

#[test]
fn nothing_to_cut_takes_no_time_and_bad_speeds_are_refused() {
    let t = estimate_time(&[], 600.0, 2.0, (0.3, 0.3), 3000.0).unwrap();
    assert_eq!((t.cutting_s, t.waiting_s, t.rapid_s), (0.0, 0.0, 0.0));
    let one = [rectangle_at(0.0)];
    for (feed, wait, rapid) in [
        (0.0, 0.0, 3000.0),
        (-1.0, 0.0, 3000.0),
        (f64::NAN, 0.0, 3000.0),
        (600.0, 0.0, 0.0),
        (600.0, 0.0, f64::INFINITY),
        (600.0, -1.0, 3000.0),
        (600.0, f64::NAN, 3000.0),
    ] {
        assert!(
            estimate_time(&one, feed, wait, (0.0, 0.0), rapid).is_err(),
            "{feed} {wait} {rapid}"
        );
    }
}

#[test]
fn a_duration_is_written_for_a_person() {
    for (seconds, text) in [
        (0.0, "0 s"),
        (0.4, "0 s"),
        (45.0, "45 s"),
        (59.5, "1 min 0 s"),
        (147.0, "2 min 27 s"),
        (3720.0, "1 h 2 min"),
        (f64::NAN, "0 s"),
        (-5.0, "0 s"),
    ] {
        assert_eq!(duration_text(seconds), text, "{seconds}");
    }
}

#[test]
fn the_rapid_speed_of_the_settings_is_zero_or_from_100_to_100000() {
    assert_eq!(parse_settings("").unwrap().rapid_mm_min, 0.0);
    assert_eq!(
        parse_settings("rapid_mm_min = 3000 # fast\n")
            .unwrap()
            .rapid_mm_min,
        3000.0
    );
    assert_eq!(parse_rapid("--rapid", "100").unwrap(), 100.0);
    assert_eq!(parse_rapid("--rapid", " 100000 ").unwrap(), 100_000.0);
    assert_eq!(parse_rapid("--rapid", "0").unwrap(), 0.0);
    for (text, line) in [
        ("rapid_mm_min = 99.9\n", 1),
        ("\nrapid_mm_min = 100001\n", 2),
        ("rapid_mm_min = -1\n", 1),
        ("rapid_mm_min = nan\n", 1),
        ("rapid_mm_min = inf\n", 1),
        ("rapid_mm_min = fast\n", 1),
        ("rapid_mm_min = 500\nrapid_mm_min = 600\n", 2),
    ] {
        let e = parse_settings(text).unwrap_err();
        assert!(e.starts_with(&format!("line {line}:")), "{text:?}: {e}");
    }
    let e = parse_rapid("--rapid", "50").unwrap_err();
    assert!(
        e.contains("--rapid") && e.contains("from 100 to 100000"),
        "{e}"
    );
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

fn convert(dir: &std::path::Path, name: &str, extra: &[&str]) -> (bool, String, String) {
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
    run(&args)
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "eh3d_stencil_estimate_{}_{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("paste.gtp"), TWO_PADS).unwrap();
    dir
}

#[test]
fn convert_prints_the_estimate_only_when_a_rapid_speed_is_given_and_the_file_is_gcode() {
    let dir = scratch("cli");
    // the default writes what it always wrote: no estimate
    let (ok, text, e) = convert(&dir, "a.nc", &[]);
    assert!(ok && !text.contains("estimated time"), "{text} {e}");
    let (ok, text, e) = convert(&dir, "b.nc", &["--rapid", "3000"]);
    assert!(ok && text.contains("estimated time: "), "{text} {e}");
    assert!(text.contains("cutting") && text.contains("waiting") && text.contains("moving"));
    // the estimate does not change the G-code
    assert_eq!(
        fs::read_to_string(dir.join("a.nc")).unwrap(),
        fs::read_to_string(dir.join("b.nc")).unwrap()
    );
    // a plotter file has no estimate
    let (ok, text, e) = convert(&dir, "c.plt", &["--rapid", "3000"]);
    assert!(ok && !text.contains("estimated time"), "{text} {e}");
    // the settings file gives the speed and the option wins over it
    let cfg = dir.join("stencil.cfg");
    let c = cfg.to_str().unwrap();
    fs::write(&cfg, "rapid_mm_min = 3000\n").unwrap();
    let (ok, text, e) = convert(&dir, "d.nc", &["--config", c]);
    assert!(ok && text.contains("estimated time: "), "{text} {e}");
    let (ok, text, e) = convert(&dir, "e.nc", &["--config", c, "--rapid", "0"]);
    assert!(ok && !text.contains("estimated time"), "{text} {e}");
    // a speed out of range is an error and nothing is written
    let (ok, _, e) = convert(&dir, "f.nc", &["--rapid", "50"]);
    assert!(
        !ok && e.contains("--rapid") && e.contains("100 to 100000"),
        "{e}"
    );
    assert!(!dir.join("f.nc").exists());
    let _ = fs::remove_dir_all(&dir);
}
