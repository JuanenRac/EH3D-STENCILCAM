//! The webs between openings, the openings that repeat another one, the paste an opening holds, the summary of `check` and `check --json`.

use eh3d_stencilcam::config::{default_file, parse, parse_min_web, Settings, DEFAULT_MIN_WEB_MM};
use eh3d_stencilcam::openings::{
    check_json, deduplicated, gap, json_string, narrow_webs, paste_volume_mm3, summary, Form,
    NarrowWeb, Opening,
};
use std::fs;
use std::process::Command;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Opening {
    Opening {
        x,
        y,
        w,
        h,
        form: Form::Rectangle,
    }
}

fn round(x: f64, y: f64, d: f64) -> Opening {
    Opening {
        x,
        y,
        w: d,
        h: d,
        form: Form::Round,
    }
}

// ---- openings that repeat another one ----

#[test]
fn the_second_of_two_equal_openings_is_left_out_and_the_first_keeps_its_place() {
    let a = rect(1.0, 1.0, 1.0, 0.5);
    let b = rect(5.0, 1.0, 1.0, 0.5);
    let (kept, removed) = deduplicated(&[a, b, a], 0.001);
    assert_eq!(removed, 1);
    assert_eq!(kept, vec![a, b]);
}

#[test]
fn a_repeated_opening_flashed_before_its_twin_in_x_still_keeps_the_first_of_the_layer() {
    // the second one is a hair to the left of the first one, within the tolerance: the first of the layer stays
    let first = rect(2.0005, 1.0, 1.0, 0.5);
    let second = rect(2.0, 1.0, 1.0, 0.5);
    let (kept, removed) = deduplicated(&[first, second], 0.001);
    assert_eq!((kept, removed), (vec![first], 1));
}

#[test]
fn openings_of_another_form_or_beyond_the_tolerance_are_not_repeats() {
    let r = round(1.0, 1.0, 0.6);
    let o = Opening {
        form: Form::Obround,
        ..r
    };
    assert_eq!(deduplicated(&[r, o], 0.001).1, 0);
    let a = rect(1.0, 1.0, 1.0, 0.5);
    let near = rect(1.01, 1.0, 1.0, 0.5);
    assert_eq!(deduplicated(&[a, near], 0.001).1, 0);
    assert_eq!(deduplicated(&[a, near], 0.02).1, 1);
    // the size counts too
    assert_eq!(deduplicated(&[a, rect(1.0, 1.0, 1.0, 0.6)], 0.001).1, 0);
}

#[test]
fn a_tolerance_that_is_not_a_number_or_is_negative_means_exactly_equal() {
    let a = rect(1.0, 1.0, 1.0, 0.5);
    let near = rect(1.0001, 1.0, 1.0, 0.5);
    for tol in [f64::NAN, -1.0, f64::NEG_INFINITY, 0.0] {
        assert_eq!(deduplicated(&[a, near], tol).1, 0, "{tol}");
        assert_eq!(deduplicated(&[a, a], tol).1, 1, "{tol}");
    }
}

#[test]
fn nothing_to_deduplicate_is_nothing_and_many_copies_leave_one() {
    assert_eq!(deduplicated(&[], 0.001), (Vec::new(), 0));
    let a = round(3.0, 3.0, 0.5);
    let (kept, removed) = deduplicated(&[a; 50], 0.001);
    assert_eq!((kept.len(), removed), (1, 49));
}

// ---- the web between two openings ----

#[test]
fn the_gap_of_two_round_openings_is_measured_between_their_circles() {
    let a = round(0.0, 0.0, 1.0);
    let b = round(2.0, 0.0, 1.0);
    assert!((gap(&a, &b) - 1.0).abs() < 1e-12);
    // on the diagonal the circles are further apart than their boxes are
    let c = round(1.0, 1.0, 1.0);
    assert!((gap(&a, &c) - (2.0f64.sqrt() - 1.0)).abs() < 1e-12);
    // touching and overlapping
    assert!(gap(&a, &round(1.0, 0.0, 1.0)).abs() < 1e-12);
    assert!(gap(&a, &round(0.5, 0.0, 1.0)) < 0.0);
}

#[test]
fn the_gap_of_rectangles_is_the_distance_of_their_nearest_sides_or_corners() {
    let a = rect(0.0, 0.0, 2.0, 1.0);
    assert!((gap(&a, &rect(3.0, 0.0, 2.0, 1.0)) - 1.0).abs() < 1e-12);
    assert!((gap(&a, &rect(0.0, 2.0, 2.0, 1.0)) - 1.0).abs() < 1e-12);
    // corner to corner: 1 mm in x and 1 mm in y
    assert!((gap(&a, &rect(3.0, 2.0, 2.0, 1.0)) - 2.0f64.sqrt()).abs() < 1e-12);
    // overlapping rectangles have a negative gap, the depth of the overlap on the shallower axis
    assert!((gap(&a, &rect(1.5, 0.0, 2.0, 1.0)) + 0.5).abs() < 1e-12);
    assert_eq!(gap(&a, &a), -1.0);
}

#[test]
fn a_web_narrower_than_the_limit_is_found_with_the_numbers_of_both_openings() {
    let o = [
        rect(0.0, 0.0, 1.0, 1.0),
        rect(1.05, 0.0, 1.0, 1.0),
        rect(5.0, 0.0, 1.0, 1.0),
    ];
    let w = narrow_webs(&o, 0.1);
    assert_eq!(w.len(), 1);
    assert_eq!((w[0].first, w[0].second), (1, 2));
    assert!((w[0].web_mm - 0.05).abs() < 1e-9);
    // a limit under the web finds nothing
    assert!(narrow_webs(&o, 0.04).is_empty());
}

#[test]
fn overlapping_openings_are_a_web_of_zero_or_less_and_no_limit_gives_no_warning() {
    let o = [round(0.0, 0.0, 1.0), round(0.5, 0.0, 1.0)];
    let w = narrow_webs(&o, 0.1);
    assert_eq!(w.len(), 1);
    assert!(w[0].web_mm < 0.0);
    for limit in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(narrow_webs(&o, limit).is_empty(), "{limit}");
    }
    assert!(narrow_webs(&[], 0.1).is_empty());
    assert!(narrow_webs(&o[..1], 0.1).is_empty());
}

#[test]
fn the_sweep_finds_the_same_pairs_as_comparing_every_pair() {
    // a deterministic pseudo-random board of 300 openings
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s % 10_000) as f64 / 10_000.0
    };
    let mut o = Vec::new();
    for i in 0..300 {
        let (x, y) = (next() * 40.0, next() * 25.0);
        let (w, h) = (0.2 + next() * 1.5, 0.2 + next() * 1.5);
        o.push(if i % 3 == 0 {
            round(x, y, w)
        } else {
            rect(x, y, w, h)
        });
    }
    for limit in [0.05, 0.3, 1.0] {
        let mut brute = Vec::new();
        for (i, a) in o.iter().enumerate() {
            for (j, b) in o.iter().enumerate().skip(i + 1) {
                let web = gap(a, b);
                if web < limit {
                    brute.push(NarrowWeb {
                        first: i + 1,
                        second: j + 1,
                        web_mm: web,
                    });
                }
            }
        }
        brute.sort_by(|a, b| a.first.cmp(&b.first).then(a.second.cmp(&b.second)));
        assert_eq!(narrow_webs(&o, limit), brute, "limit {limit}");
    }
}

// ---- the paste ----

#[test]
fn the_paste_of_an_opening_is_its_area_times_the_thickness() {
    assert!((paste_volume_mm3(&rect(0.0, 0.0, 2.0, 1.0), 0.12) - 0.24).abs() < 1e-12);
    let r = round(0.0, 0.0, 1.0);
    assert!((paste_volume_mm3(&r, 0.1) - std::f64::consts::PI / 4.0 * 0.1).abs() < 1e-12);
}

#[test]
fn the_summary_counts_the_forms_and_adds_the_paste() {
    let o = [
        round(0.0, 0.0, 1.0),
        rect(3.0, 0.0, 2.0, 1.0),
        rect(6.0, 0.0, 1.0, 1.0),
        Opening {
            form: Form::Obround,
            ..rect(9.0, 0.0, 2.0, 1.0)
        },
    ];
    let s = summary(&o, 0.1);
    assert_eq!((s.round, s.rectangle, s.obround), (1, 2, 1));
    let want = std::f64::consts::PI / 4.0 * 0.1 + 2.0 * 0.1 + 1.0 * 0.1 + 2.0 * 0.1;
    assert!((s.paste_ul - want).abs() < 1e-12, "{}", s.paste_ul);
    let none = summary(&[], 0.1);
    assert_eq!(
        (none.round, none.rectangle, none.obround, none.paste_ul),
        (0, 0, 0, 0.0)
    );
}

// ---- the JSON ----

#[test]
fn a_text_is_written_as_a_json_string() {
    assert_eq!(json_string("plain"), "\"plain\"");
    assert_eq!(json_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
    assert_eq!(json_string("a\nb\r\t"), "\"a\\nb\\r\\t\"");
    assert_eq!(json_string("\u{1}"), "\"\\u0001\"");
    assert_eq!(json_string("año ✓"), "\"año ✓\"");
}

#[test]
fn the_json_of_a_check_has_every_field_and_null_ratios_when_there_are_no_openings() {
    let o = [rect(0.0, 0.0, 1.0, 0.5), round(3.0, 0.0, 0.6)];
    let weak = vec![(1usize, "it \"sticks\"".to_string())];
    let webs = vec![NarrowWeb {
        first: 1,
        second: 2,
        web_mm: 0.05,
    }];
    let j = check_json(&o, 0.1, 10.0, &weak, &webs, 3).unwrap();
    for part in [
        "\"openings\":2",
        "\"round\":1",
        "\"rectangle\":1",
        "\"obround\":0",
        "\"thickness_mm\":0.1",
        "\"shrink_percent\":10",
        "\"weak\":[{\"opening\":2,\"message\":\"it \\\"sticks\\\"\"}]",
        "\"narrow_webs\":[{\"first\":1,\"second\":2,\"web_mm\":0.05}]",
        "\"duplicates_removed\":3}",
    ] {
        assert!(j.contains(part), "{part} in {j}");
    }
    assert!(j.starts_with('{') && !j.contains('\n'));
    let empty = check_json(&[], 0.1, 10.0, &[], &[], 0).unwrap();
    assert!(
        empty.contains("\"min_aspect\":null,\"min_area\":null"),
        "{empty}"
    );
    assert!(empty.contains("\"weak\":[],\"narrow_webs\":[]"), "{empty}");
    assert!(check_json(&o, 0.0, 10.0, &[], &[], 0).is_err());
}

// ---- the setting ----

#[test]
fn the_narrowest_web_is_a_setting_with_a_default_a_range_and_a_line_in_the_file() {
    assert_eq!(Settings::default().min_web_mm, DEFAULT_MIN_WEB_MM);
    assert_eq!(parse("min_web_mm = 0.25\n").unwrap().min_web_mm, 0.25);
    assert_eq!(parse("min_web_mm = 0 # off\n").unwrap().min_web_mm, 0.0);
    assert_eq!(parse_min_web("--min-web", " 5 ").unwrap(), 5.0);
    for bad in ["-0.1", "5.1", "x", "NaN", "inf", ""] {
        let e = parse_min_web("--min-web", bad).unwrap_err();
        assert!(
            e.contains("--min-web") && e.contains("0 to 5"),
            "{bad}: {e}"
        );
    }
    let e = parse("min_web_mm = 9\n").unwrap_err();
    assert!(e.starts_with("line 1:") && e.contains("min_web_mm"), "{e}");
    let e = parse("min_web_mm = 1\nmin_web_mm = 2\n").unwrap_err();
    assert!(e.contains("line 2") && e.contains("twice"), "{e}");
    assert!(default_file().contains("\nmin_web_mm = 0.1\n"));
}

// ---- the command line ----

const CLOSE: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX30500Y20000D03*\nX90000Y20000D03*\nM02*\n";
const TWICE: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX20000Y20000D03*\nX60000Y20000D03*\nM02*\n";

fn run(args: &[&str]) -> (Option<i32>, String, String) {
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

fn layer(name: &str, text: &str) -> (std::path::PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_webs_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("paste.gtp");
    fs::write(&file, text).unwrap();
    let s = file.to_str().unwrap().to_string();
    (dir, s)
}

#[test]
fn check_warns_of_a_narrow_web_and_the_exit_code_stays_clean() {
    let (dir, l) = layer("warn", CLOSE);
    let (code, out, err) = run(&["check", &l, "--shrink", "0", "--thickness", "0.1"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        err.contains("warning: openings 1 and 2 are 0.050 mm apart (less than 0.1 mm)"),
        "{err}"
    );
    assert!(!err.contains("openings 2 and 3"), "{err}");
    assert!(
        out.contains("0 round, 3 rectangular, 0 obround; the openings hold 0.150 uL of paste"),
        "{out}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_limit_of_the_web_can_be_raised_or_switched_off_and_convert_is_never_stopped_by_it() {
    let (dir, l) = layer("limit", CLOSE);
    let (_, _, err) = run(&[
        "check",
        &l,
        "--shrink",
        "0",
        "--thickness",
        "0.1",
        "--min-web",
        "0",
    ]);
    assert!(!err.contains("warning"), "{err}");
    let (_, _, err) = run(&[
        "check",
        &l,
        "--shrink",
        "0",
        "--thickness",
        "0.1",
        "--min-web",
        "5",
    ]);
    assert!(
        err.contains("openings 1 and 2") && err.contains("openings 2 and 3"),
        "{err}"
    );
    let nc = dir.join("out.nc");
    let (code, _, err) = run(&[
        "convert",
        &l,
        nc.to_str().unwrap(),
        "--shrink",
        "0",
        "--thickness",
        "0.1",
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert!(err.contains("may break") && nc.exists());
    let (code, _, err) = run(&["check", &l, "--min-web", "6"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("--min-web") && err.contains("0 to 5"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_limit_of_the_web_can_come_from_the_settings_file_and_the_option_wins() {
    let (dir, l) = layer("cfg", CLOSE);
    let cfg = dir.join("stencil.cfg");
    fs::write(&cfg, "min_web_mm = 0\n").unwrap();
    let base = [
        "check",
        l.as_str(),
        "--shrink",
        "0",
        "--thickness",
        "0.1",
        "--config",
        cfg.to_str().unwrap(),
    ];
    let (_, _, err) = run(&base);
    assert!(!err.contains("warning"), "{err}");
    let mut with = base.to_vec();
    with.extend(["--min-web", "0.2"]);
    let (_, _, err) = run(&with);
    assert!(err.contains("openings 1 and 2"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_opening_flashed_twice_is_cut_once_and_said() {
    let (dir, l) = layer("twice", TWICE);
    let nc = dir.join("out.nc");
    let (code, out, err) = run(&[
        "convert",
        &l,
        nc.to_str().unwrap(),
        "--shrink",
        "0",
        "--thickness",
        "0.1",
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        err.contains("note: 1 opening(s) repeat another one"),
        "{err}"
    );
    assert!(out.contains("2 openings written"), "{out}");
    let (_, out, _) = run(&["check", &l, "--shrink", "0", "--thickness", "0.1"]);
    assert!(out.starts_with("2 openings;"), "{out}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn check_json_prints_one_object_and_only_check_takes_it() {
    let (dir, l) = layer("json", CLOSE);
    let (code, out, err) = run(&["check", &l, "--shrink", "0", "--thickness", "0.1", "--json"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("\"openings\":3") && out.contains("\"rectangle\":3"),
        "{out}"
    );
    assert!(
        out.contains("\"narrow_webs\":[{\"first\":1,\"second\":2"),
        "{out}"
    );
    assert!(
        out.contains("\"weak\":[]") && out.contains("\"duplicates_removed\":0"),
        "{out}"
    );
    let nc = dir.join("out.nc");
    let (code, _, err) = run(&["convert", &l, nc.to_str().unwrap(), "--json"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("--json is an option of check"), "{err}");
    assert!(!nc.exists());
    // a thick sheet makes the openings weak: the exit code says it and the object lists them
    let (code, out, _) = run(&["check", &l, "--shrink", "0", "--thickness", "0.5", "--json"]);
    assert_eq!(code, Some(2));
    assert!(out.contains("\"weak\":[{\"opening\":1"), "{out}");
    let _ = fs::remove_dir_all(&dir);
}
