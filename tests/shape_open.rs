//! The shapes of a paste layer that are not flashed pads: a macro pad that is a rectangle, a rectangle drawn as a region, and a track along an axis become openings; the rest are notes
//! with their reason, and a region that nearly fills its rectangle is opened as that rectangle only when it is asked for.

use eh3d_stencilcam::openings::{
    from_gerber_rules, from_gerber_with, Form, Opening, ShapeRules, MAX_REGION_FIT_PERCENT,
};
use std::fs;
use std::process::Command;

const HEAD: &str = "%FSLAX24Y24*%\n%MOMM*%\n";

fn parse(text: &str) -> eh3d_parser_gerber::GerberFile {
    eh3d_parser_gerber::parse(text)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// A macro that holds one outline of the given corners (the last one repeats the first).
fn outline_macro(points: &[(f64, f64)]) -> String {
    let list: Vec<String> = points.iter().map(|(x, y)| format!("{x},{y}")).collect();
    format!(
        "{HEAD}%AMSHAPE*\n4,1,{},{},0*%\n%ADD10SHAPE*%\nD10*\nX20000Y30000D03*\nM02*\n",
        points.len() - 1,
        list.join(",")
    )
}

const CLOSED_BOX: [(f64, f64); 5] = [
    (-0.5, -0.5),
    (0.5, -0.5),
    (0.5, 0.5),
    (-0.5, 0.5),
    (-0.5, -0.5),
];

/// A 2 x 1 pad with its four corners cut by 0.1 mm: it fills 99 % of its enclosing rectangle.
const CUT_CORNERS: [(f64, f64); 9] = [
    (-0.9, -0.5),
    (0.9, -0.5),
    (1.0, -0.4),
    (1.0, 0.4),
    (0.9, 0.5),
    (-0.9, 0.5),
    (-1.0, 0.4),
    (-1.0, -0.4),
    (-0.9, -0.5),
];

#[test]
fn a_macro_pad_becomes_an_opening() {
    let (o, notes) = from_gerber_with(&parse(&outline_macro(&CLOSED_BOX)), 0.005);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(o.len(), 1, "{o:?}");
    assert_eq!(o[0].form, Form::Rectangle);
    assert!(close(o[0].x, 2.0) && close(o[0].y, 3.0), "{:?}", o[0]);
    assert!(close(o[0].w, 1.0) && close(o[0].h, 1.0), "{:?}", o[0]);
}

#[test]
fn a_rectangle_drawn_as_a_region_becomes_an_opening() {
    let text = format!("{HEAD}G36*\nX0Y0D02*\nX30000Y0D01*\nX30000Y10000D01*\nX0Y10000D01*\nX0Y0D01*\nG37*\nM02*\n");
    let (o, notes) = from_gerber_with(&parse(&text), 0.005);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(o.len(), 1, "{o:?}");
    assert!(close(o[0].x, 1.5) && close(o[0].y, 0.5));
    assert!(close(o[0].w, 3.0) && close(o[0].h, 1.0));
}

#[test]
fn a_pad_with_cut_corners_needs_the_fit_and_is_then_reported_apart() {
    let file = parse(&outline_macro(&CUT_CORNERS));
    let none = from_gerber_rules(&file, 0.005, &ShapeRules::default());
    assert!(none.openings.is_empty(), "{none:?}");
    assert_eq!(none.notes.len(), 1, "{:?}", none.notes);
    assert!(
        none.notes[0].contains("is not a rectangle") && none.notes[0].contains("99.0 %"),
        "{:?}",
        none.notes
    );
    let tight = from_gerber_rules(
        &file,
        0.005,
        &ShapeRules {
            region_fit_percent: 0.5,
        },
    );
    assert!(
        tight.openings.is_empty() && tight.notes.len() == 1,
        "{tight:?}"
    );
    let fit = from_gerber_rules(
        &file,
        0.005,
        &ShapeRules {
            region_fit_percent: 5.0,
        },
    );
    assert!(fit.notes.is_empty(), "{:?}", fit.notes);
    assert_eq!(fit.openings.len(), 1);
    assert!(close(fit.openings[0].w, 2.0) && close(fit.openings[0].h, 1.0));
    assert_eq!(fit.approximated.len(), 1);
    assert!(
        fit.approximated[0].contains("region_fit_percent 5"),
        "{:?}",
        fit.approximated
    );
    // a pad that is exactly a rectangle is not an approximation
    let exact = from_gerber_rules(
        &parse(&outline_macro(&CLOSED_BOX)),
        0.005,
        &ShapeRules {
            region_fit_percent: 5.0,
        },
    );
    assert!(exact.approximated.is_empty() && exact.openings.len() == 1);
}

#[test]
fn tracks_along_an_axis_become_obround_openings_and_a_point_a_round_one() {
    let text = format!("{HEAD}%ADD11C,0.2*%\nD11*\nX0Y0D02*\nX30000Y0D01*\nX50000Y10000D02*\nX50000Y40000D01*\nX70000Y70000D02*\nX70000Y70000D01*\nM02*\n");
    let (o, notes) = from_gerber_with(&parse(&text), 0.005);
    assert!(notes.is_empty(), "{notes:?}");
    let obrounds: Vec<&Opening> = o.iter().filter(|p| p.form == Form::Obround).collect();
    assert_eq!(obrounds.len(), 2, "{o:?}");
    let flat = obrounds
        .iter()
        .find(|p| p.w > p.h)
        .expect("the track along x");
    assert!(
        close(flat.x, 1.5) && close(flat.y, 0.0) && close(flat.w, 3.2) && close(flat.h, 0.2),
        "{flat:?}"
    );
    let tall = obrounds
        .iter()
        .find(|p| p.h > p.w)
        .expect("the track along y");
    assert!(
        close(tall.x, 5.0) && close(tall.y, 2.5) && close(tall.w, 0.2) && close(tall.h, 3.2),
        "{tall:?}"
    );
}

#[test]
fn a_track_at_an_angle_is_a_note_with_its_reason() {
    let text = format!("{HEAD}%ADD11C,0.2*%\nD11*\nX0Y0D02*\nX30000Y10000D01*\nM02*\n");
    let (o, notes) = from_gerber_with(&parse(&text), 0.005);
    assert!(o.is_empty(), "{o:?}");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(
        notes[0].contains("track at an angle")
            && notes[0].contains("(0.000, 0.000) to (3.000, 1.000)"),
        "{notes:?}"
    );
}

#[test]
fn the_reasons_are_written_up_to_five_and_the_rest_are_counted() {
    let mut text = format!("{HEAD}%ADD11C,0.2*%\nD11*\n");
    for k in 0..8 {
        text.push_str(&format!(
            "X{}Y0D02*\nX{}Y10000D01*\n",
            k * 20000,
            k * 20000 + 10000
        ));
    }
    text.push_str("M02*\n");
    let (o, notes) = from_gerber_with(&parse(&text), 0.005);
    assert!(o.is_empty());
    assert_eq!(notes.len(), 6, "{notes:?}");
    assert!(
        notes[5].starts_with("3 more tracks or regions"),
        "{notes:?}"
    );
}

#[test]
fn a_region_that_is_not_a_rectangle_says_how_much_it_fills() {
    let text = format!("{HEAD}G36*\nX0Y0D02*\nX20000Y0D01*\nX0Y20000D01*\nX0Y0D01*\nG37*\nM02*\n");
    let (o, notes) = from_gerber_with(&parse(&text), 0.005);
    assert!(o.is_empty());
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(
        notes[0].contains("region of 3 corners") && notes[0].contains("50.0 %"),
        "{notes:?}"
    );
}

#[test]
fn the_fit_percent_is_checked_against_its_range() {
    assert!(ShapeRules::default().validate().is_ok());
    assert!(ShapeRules {
        region_fit_percent: MAX_REGION_FIT_PERCENT
    }
    .validate()
    .is_ok());
    for bad in [-0.1, MAX_REGION_FIT_PERCENT + 0.1, f64::NAN, f64::INFINITY] {
        assert!(
            ShapeRules {
                region_fit_percent: bad
            }
            .validate()
            .is_err(),
            "{bad}"
        );
    }
}

/// A small generator with a fixed seed: the same numbers on every run.
struct Seed(u64);

impl Seed {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[test]
fn damaged_shape_layers_are_read_without_a_panic_and_give_sound_openings() {
    let mut seed = Seed(0x5EED_0030);
    let alphabet: Vec<&str> = vec![
        "G36*\n",
        "G37*\n",
        "D01*\n",
        "D02*\n",
        "D03*\n",
        "%ADD11C,0.2*%\n",
        "%ADD12R,1X0.5*%\n",
        "D11*\n",
        "D12*\n",
        "%AMX*4,1,3,0,0,1,0,1,1,0,0,0*%\n",
        "%ADD13X*%\n",
        "D13*\n",
        "X99999999Y0D01*\n",
        "X-1Y-1D02*\n",
        "G01*\n",
        "G02*\n",
        "X10000Y10000I5000J0D01*\n",
        "%AMB*4,1,2,1e308,0,-1e308,0,1e308,0,0*%\n",
        "garbage\n",
        "M02*\n",
    ];
    for round in 0..300 {
        let mut text = String::from(HEAD);
        for _ in 0..seed.below(40) {
            if seed.below(3) == 0 {
                text.push_str(&format!(
                    "X{}Y{}D0{}*\n",
                    seed.below(200_000),
                    seed.below(200_000),
                    1 + seed.below(3)
                ));
            } else {
                text.push_str(alphabet[seed.below(alphabet.len() as u64) as usize]);
            }
        }
        let rules = ShapeRules {
            region_fit_percent: seed.below(21) as f64,
        };
        let shaped = from_gerber_rules(&parse(&text), 0.005, &rules);
        assert!(
            shaped.notes.len() <= 40,
            "round {round}: {} notes",
            shaped.notes.len()
        );
        for o in &shaped.openings {
            assert!(
                o.x.is_finite() && o.y.is_finite() && o.w.is_finite() && o.h.is_finite(),
                "round {round}: {o:?}"
            );
            assert!(o.w > 0.0 && o.h > 0.0, "round {round}: {o:?}");
        }
    }
}

#[test]
fn a_layer_with_a_pad_with_cut_corners_is_refused_with_code_two_unless_the_fit_allows_it() {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_shape_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let layer = dir.join("paste.gtp");
    let out = dir.join("out.nc");
    fs::write(&layer, outline_macro(&CUT_CORNERS)).unwrap();
    let (layer_s, out_s) = (layer.to_str().unwrap(), out.to_str().unwrap());
    let run = |extra: &[&str]| {
        let mut args = vec!["convert", layer_s, out_s, "--thickness", "0.1"];
        args.extend_from_slice(extra);
        let o = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
            .args(&args)
            .output()
            .expect("the program runs");
        (
            o.status.code(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let (code, why) = run(&[]);
    assert_eq!(code, Some(2), "{why}");
    assert!(why.contains("is not a rectangle") && !out.exists(), "{why}");
    let (code, why) = run(&["--region-fit", "5"]);
    assert_eq!(code, Some(0), "{why}");
    assert!(why.contains("enclosing rectangle") && out.exists(), "{why}");
    let (code, why) = run(&["--region-fit", "21"]);
    assert_eq!(code, Some(1), "{why}");
    let _ = fs::remove_dir_all(&dir);
}
