//! The limits under which an opening does not release the paste are settings (`min_aspect` and `min_area`), each reason names the ratio, its limit and the thickest sheet that would do,
//! and a check that did not pass ends with the code 2.

use eh3d_stencilcam::config::{self, Settings};
use eh3d_stencilcam::openings::{
    max_thickness_mm, ratios, weak, weak_with, Form, Limits, Opening, MIN_AREA, MIN_ASPECT,
};
use std::fs;
use std::process::Command;

fn pad(w: f64, h: f64, form: Form) -> Opening {
    Opening {
        x: 0.0,
        y: 0.0,
        w,
        h,
        form,
    }
}

#[test]
fn the_default_limits_are_the_guideline_and_weak_uses_them() {
    let l = Limits::default();
    assert_eq!((l.min_aspect, l.min_area), (MIN_ASPECT, MIN_AREA));
    let pads = [
        pad(0.2, 0.2, Form::Rectangle),
        pad(1.0, 1.0, Form::Rectangle),
    ];
    assert_eq!(
        weak(&pads, 0.12).unwrap(),
        weak_with(&pads, 0.12, &l).unwrap()
    );
}

#[test]
fn a_stricter_limit_makes_more_openings_weak_and_the_reason_names_it() {
    let pads = [pad(0.34, 0.34, Form::Rectangle), pad(0.5, 0.5, Form::Round)];
    let base = weak_with(&pads, 0.12, &Limits::default()).unwrap();
    let strict = weak_with(
        &pads,
        0.12,
        &Limits {
            min_aspect: 3.0,
            min_area: 0.66,
        },
    )
    .unwrap();
    assert!(strict.len() > base.len(), "{base:?} {strict:?}");
    assert!(
        strict[0].1.contains("aspect ratio") && strict[0].1.contains("below 3;"),
        "{:?}",
        strict[0]
    );
    assert!(strict[0].1.contains("mm thick would do"), "{:?}", strict[0]);
    let by_area = weak_with(
        &[pad(0.5, 0.5, Form::Rectangle)],
        0.12,
        &Limits {
            min_aspect: 1.0,
            min_area: 1.2,
        },
    )
    .unwrap();
    assert!(
        by_area[0].1.contains("area ratio") && by_area[0].1.contains("below 1.2;"),
        "{by_area:?}"
    );
    // a limit outside its range is an error, not a silent value
    assert!(weak_with(
        &pads,
        0.12,
        &Limits {
            min_aspect: 0.0,
            min_area: 0.66
        }
    )
    .is_err());
    assert!(weak_with(
        &pads,
        0.12,
        &Limits {
            min_aspect: 1.5,
            min_area: f64::NAN
        }
    )
    .is_err());
}

struct Seed(u64);

impl Seed {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn the_thickest_sheet_that_does_for_an_opening_meets_both_limits_and_a_thicker_one_does_not() {
    let mut s = Seed(0x5EED_0031);
    for k in 0..400 {
        let (w, h) = (0.05 + s.unit() * 3.0, 0.05 + s.unit() * 3.0);
        let form = [Form::Rectangle, Form::Round, Form::Obround][k % 3];
        let o = if form == Form::Round {
            pad(w, w, form)
        } else {
            pad(w, h, form)
        };
        let limits = Limits {
            min_aspect: 0.5 + s.unit() * 4.0,
            min_area: 0.1 + s.unit() * 1.5,
        };
        let t = max_thickness_mm(&o, &limits);
        assert!(t > 0.0 && t.is_finite(), "{o:?} {limits:?} {t}");
        let at = ratios(&o, t).unwrap();
        assert!(
            at.aspect >= limits.min_aspect - 1e-9 && at.area >= limits.min_area - 1e-9,
            "{o:?} {limits:?} {at:?}"
        );
        let over = ratios(&o, t * 1.01).unwrap();
        assert!(
            over.aspect < limits.min_aspect || over.area < limits.min_area,
            "{o:?} {limits:?} {over:?}"
        );
    }
}

#[test]
fn the_limits_are_read_from_the_file_with_their_ranges_and_written_back_by_the_default_file() {
    let s = config::parse("min_aspect = 2.5\nmin_area = 0.8\nregion_fit_percent = 4\n").unwrap();
    assert_eq!(
        (s.min_aspect, s.min_area, s.region_fit_percent),
        (2.5, 0.8, 4.0)
    );
    assert_eq!(
        s.limits(),
        Limits {
            min_aspect: 2.5,
            min_area: 0.8
        }
    );
    assert_eq!(s.shape_rules().region_fit_percent, 4.0);
    for (text, why) in [
        (
            "min_aspect = 0.4\n",
            "line 1: min_aspect '0.4' must be a number from 0.5 to 10",
        ),
        ("min_aspect = 10.5\n", "from 0.5 to 10"),
        (
            "min_area = 0.05\n",
            "line 1: min_area '0.05' must be a number from 0.1 to 2",
        ),
        ("min_area = x\n", "min_area 'x'"),
        ("region_fit_percent = 21\n", "from 0 to 20"),
        ("region_fit_percent = -1\n", "from 0 to 20"),
        (
            "min_area = 1\nmin_area = 1\n",
            "line 2: min_area is given twice",
        ),
    ] {
        let e = config::parse(text).unwrap_err();
        assert!(e.contains(why), "{text:?}: {e}");
    }
    assert_eq!(
        config::parse(&config::default_file()).unwrap(),
        Settings::default()
    );
    assert!(
        config::default_file().contains("min_aspect = 1.5")
            && config::default_file().contains("min_area = 0.66")
    );
}

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

const PADS: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,0.5X0.5*%\nD10*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";

#[test]
fn a_check_that_did_not_pass_ends_with_code_two_and_the_limits_can_be_moved() {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_limits_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let layer = dir.join("paste.gtp");
    fs::write(&layer, PADS).unwrap();
    let l = layer.to_str().unwrap();
    // 0.5 mm pads shrunk by 10 % on 0.12 mm: aspect 3.75, area 0.94 (passes with the defaults)
    let (code, out, err) = run(&["check", l, "--thickness", "0.12"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(
        out.contains("(at least 1.5)") && out.contains("(at least 0.66)"),
        "{out}"
    );
    let (code, out, _) = run(&["check", l, "--thickness", "0.12", "--min-aspect", "4"]);
    assert_eq!(code, Some(2));
    assert!(
        out.contains("aspect ratio 3.75 is below 4;") && out.contains("(at least 4)"),
        "{out}"
    );
    let (code, out, _) = run(&["check", l, "--thickness", "0.12", "--min-area", "1"]);
    assert_eq!(code, Some(2));
    assert!(out.contains("area ratio 0.94 is below 1;"), "{out}");
    // the file moves the limit and the option wins over the file
    let cfg = dir.join("stencil.cfg");
    fs::write(&cfg, "min_aspect = 4\n").unwrap();
    let c = cfg.to_str().unwrap();
    assert_eq!(
        run(&["check", l, "--thickness", "0.12", "--config", c]).0,
        Some(2)
    );
    assert_eq!(
        run(&[
            "check",
            l,
            "--thickness",
            "0.12",
            "--config",
            c,
            "--min-aspect",
            "1.5"
        ])
        .0,
        Some(0)
    );
    // convert refuses with the same code and writes nothing; --force writes
    let out_file = dir.join("out.nc");
    let o = out_file.to_str().unwrap();
    let (code, _, err) = run(&["convert", l, o, "--thickness", "0.12", "--min-aspect", "4"]);
    assert_eq!(code, Some(2), "{err}");
    assert!(!out_file.exists());
    assert_eq!(
        run(&[
            "convert",
            l,
            o,
            "--thickness",
            "0.12",
            "--min-aspect",
            "4",
            "--force"
        ])
        .0,
        Some(0)
    );
    // a limit outside its range is a mistake in what was asked (1), not a failed check
    let (code, _, err) = run(&["check", l, "--min-aspect", "11"]);
    assert_eq!(code, Some(1), "{err}");
    assert!(
        err.contains("--min-aspect '11' must be a number from 0.5 to 10"),
        "{err}"
    );
    assert_eq!(run(&["check", l, "--min-area", "0"]).0, Some(1));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_json_of_check_still_ends_with_code_two_when_an_opening_is_weak() {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_limits_json_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let layer = dir.join("paste.gtp");
    fs::write(&layer, PADS).unwrap();
    let (code, out, _) = run(&[
        "check",
        layer.to_str().unwrap(),
        "--thickness",
        "0.12",
        "--min-aspect",
        "4",
        "--json",
    ]);
    assert_eq!(code, Some(2));
    assert!(out.contains("\"weak\":[{\"opening\":1"), "{out}");
    let _ = fs::remove_dir_all(&dir);
}
