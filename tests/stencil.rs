use eh3d_stencilcam::openings::{
    adjusted, from_gerber, mirrored, ratios, shrunk, weak, Form, Opening, MIN_ASPECT,
};
use eh3d_stencilcam::output::{gcode, hpgl, ordered, outline, paths};
use eh3d_stencilcam::profile::{Material, Profile};

const PASTE: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\n%ADD11C,0.6*%\n%ADD12O,2.0X1.0*%\nD10*\nX20000Y20000D03*\nX40000Y20000D03*\nD11*\nX60000Y20000D03*\nD12*\nX80000Y20000D03*\nM02*\n";

fn rect(x: f64, y: f64, w: f64, h: f64) -> Opening {
    Opening {
        x,
        y,
        w,
        h,
        form: Form::Rectangle,
    }
}

fn profile() -> Profile {
    Profile {
        thickness_mm: 0.1,
        shrink_percent: 10.0,
        corner_radius_mm: 0.05,
        kerf_mm: 0.06,
    }
}

#[test]
fn the_pads_of_the_paste_layer_become_openings_in_millimetres() {
    let (o, notes) = from_gerber(&eh3d_parser_gerber::parse(PASTE));
    assert_eq!(o.len(), 4);
    assert!(notes.is_empty());
    assert_eq!(
        (o[0].x, o[0].y, o[0].w, o[0].h, o[0].form),
        (2.0, 2.0, 1.0, 0.5, Form::Rectangle)
    );
    assert_eq!((o[2].form, o[2].w, o[2].h), (Form::Round, 0.6, 0.6));
    assert_eq!(o[3].form, Form::Obround);
}

#[test]
fn inches_and_what_is_not_a_pad_are_handled() {
    let text = "%FSLAX24Y24*%\n%MOIN*%\n%ADD10R,0.04X0.02*%\n%ADD11P,0.1X6*%\nD10*\nX10000Y10000D03*\nD11*\nX0Y0D03*\nX0Y0D02*\nX10000Y0D01*\nM02*\n";
    let (o, notes) = from_gerber(&eh3d_parser_gerber::parse(text));
    // the polygon is drawn as a circle by the conversion of the suite, and it is said
    assert_eq!(o.len(), 2);
    assert!((o[0].x - 25.4).abs() < 1e-9 && (o[0].w - 1.016).abs() < 1e-9);
    assert_eq!((o[1].form, o[1].w), (Form::Round, 2.54));
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert!(notes.iter().any(|n| n.contains("polygon")), "{notes:?}");
    assert!(from_gerber(&eh3d_parser_gerber::parse("")).0.is_empty());
    assert_eq!(
        from_gerber(&eh3d_parser_gerber::parse(
            "%FSLAX24Y24*%\n%MOMM*%\nX0Y0D03*\nM02*\n"
        ))
        .1
        .len(),
        1
    );
}

#[test]
fn the_opening_is_shrunk_and_then_the_kerf_is_taken_off_the_cut_line() {
    let o = rect(0.0, 0.0, 1.0, 0.5);
    let s = shrunk(&o, &profile()).unwrap();
    assert!((s.w - 0.9).abs() < 1e-9 && (s.h - 0.45).abs() < 1e-9);
    let a = adjusted(&o, &profile()).unwrap();
    assert!(
        (a.w - 0.84).abs() < 1e-9 && (a.h - 0.39).abs() < 1e-9,
        "the beam removes 0.03 on each side: {a:?}"
    );
    assert_eq!((a.x, a.y, a.form), (0.0, 0.0, Form::Rectangle));
}

#[test]
fn an_opening_that_disappears_is_an_error_and_a_bad_profile_too() {
    assert!(adjusted(&rect(0.0, 0.0, 0.05, 0.05), &profile())
        .unwrap_err()
        .contains("is gone"));
    for bad in [
        Profile {
            thickness_mm: 0.0,
            ..profile()
        },
        Profile {
            shrink_percent: 60.0,
            ..profile()
        },
        Profile {
            kerf_mm: f64::NAN,
            ..profile()
        },
        Profile {
            corner_radius_mm: -1.0,
            ..profile()
        },
    ] {
        assert!(shrunk(&rect(0.0, 0.0, 1.0, 1.0), &bad).is_err(), "{bad:?}");
    }
}

#[test]
fn the_ratios_decide_if_the_paste_comes_out() {
    // a 1.0 x 0.5 opening in a sheet of 0.1: aspect 5, area ratio 0.5 / (2 * 1.5 * 0.1) = 1.67
    let r = ratios(&rect(0.0, 0.0, 1.0, 0.5), 0.1).unwrap();
    assert!((r.aspect - 5.0).abs() < 1e-9 && (r.area - 0.5 / 0.3).abs() < 1e-9);
    let round = ratios(
        &Opening {
            x: 0.0,
            y: 0.0,
            w: 0.4,
            h: 0.4,
            form: Form::Round,
        },
        0.1,
    )
    .unwrap();
    assert!(
        (round.aspect - 4.0).abs() < 1e-9
            && (round.area - 0.25 * 0.4 / 0.1 / 2.0 * 2.0 / 2.0).abs() < 1.0,
        "{round:?}"
    );
    let thin = weak(&[rect(0.0, 0.0, 0.1, 5.0), rect(1.0, 0.0, 1.0, 1.0)], 0.1).unwrap();
    assert_eq!(thin.len(), 1);
    assert_eq!(thin[0].0, 0);
    assert!(thin[0].1.contains("aspect ratio") && thin[0].1.contains(&MIN_ASPECT.to_string()));
    assert!(
        ratios(&rect(0.0, 0.0, 1.0, 1.0), 0.0).is_err()
            && ratios(&rect(0.0, 0.0, 1.0, 1.0), f64::NAN).is_err()
    );
}

#[test]
fn a_thicker_sheet_makes_more_openings_weak() {
    let pads: Vec<Opening> = (1..=6)
        .map(|i| rect(f64::from(i), 0.0, 0.1 * f64::from(i), 0.1 * f64::from(i)))
        .collect();
    let thin = weak(&pads, 0.05).unwrap().len();
    let thick = weak(&pads, 0.2).unwrap().len();
    assert!(thick > thin, "{thick} should be more than {thin}");
}

#[test]
fn mirroring_flips_the_layer_about_its_middle() {
    let m = mirrored(&[rect(1.0, 5.0, 2.0, 1.0), rect(9.0, 5.0, 2.0, 1.0)]);
    assert_eq!((m[0].x, m[1].x), (9.0, 1.0));
    assert!(mirrored(&[]).is_empty());
    assert_eq!(
        mirrored(&mirrored(&[
            rect(1.0, 5.0, 2.0, 1.0),
            rect(9.0, 5.0, 2.0, 1.0)
        ])),
        vec![rect(1.0, 5.0, 2.0, 1.0), rect(9.0, 5.0, 2.0, 1.0)]
    );
}

#[test]
fn an_outline_is_closed_has_the_size_of_the_opening_and_round_corners() {
    let o = rect(10.0, 20.0, 2.0, 1.0);
    let p = outline(&o, 0.2);
    assert_eq!(p.first(), p.last());
    let (min_x, max_x) = (
        p.iter().map(|q| q.0).fold(f64::INFINITY, f64::min),
        p.iter().map(|q| q.0).fold(f64::NEG_INFINITY, f64::max),
    );
    let (min_y, max_y) = (
        p.iter().map(|q| q.1).fold(f64::INFINITY, f64::min),
        p.iter().map(|q| q.1).fold(f64::NEG_INFINITY, f64::max),
    );
    assert!((max_x - min_x - 2.0).abs() < 1e-9 && (max_y - min_y - 1.0).abs() < 1e-9);
    assert!(
        !p.iter()
            .any(|q| (q.0 - 11.0).abs() < 1e-9 && (q.1 - 20.5).abs() < 1e-9),
        "the corner itself is cut off by the radius"
    );
    let sharp = outline(&o, 0.0);
    assert!(sharp
        .iter()
        .any(|q| (q.0 - 11.0).abs() < 1e-9 && (q.1 - 20.5).abs() < 1e-9));
    let huge = outline(&o, 99.0);
    assert!(
        huge.iter().all(|q| q.0 >= 9.0 - 1e-9 && q.0 <= 11.0 + 1e-9),
        "a radius larger than the opening is cut down to half of the small side"
    );
}

#[test]
fn a_round_opening_is_a_circle_of_its_diameter() {
    let p = outline(
        &Opening {
            x: 5.0,
            y: 5.0,
            w: 1.0,
            h: 1.0,
            form: Form::Round,
        },
        0.0,
    );
    assert!(p.iter().all(
        |q| ((q.0 - 5.0).powi(2) + (q.1 - 5.0).powi(2)).sqrt() - 0.5 < 1e-9
            && ((q.0 - 5.0).powi(2) + (q.1 - 5.0).powi(2)).sqrt() > 0.5 - 1e-9
    ));
}

#[test]
fn the_paths_are_walked_nearest_first() {
    let all = paths(
        &[
            rect(50.0, 0.0, 1.0, 1.0),
            rect(1.0, 0.0, 1.0, 1.0),
            rect(25.0, 0.0, 1.0, 1.0),
        ],
        &profile(),
    )
    .unwrap();
    let o = ordered(&all);
    assert_eq!(o.len(), 3);
    let xs: Vec<f64> = o.iter().map(|p| p[0].0.round()).collect();
    assert!(xs[0] < xs[1] && xs[1] < xs[2], "{xs:?}");
    assert!(ordered(&[]).is_empty());
}

#[test]
fn the_gcode_turns_the_laser_off_to_travel() {
    let all = paths(&[rect(5.0, 5.0, 1.0, 1.0)], &profile()).unwrap();
    let g = gcode(&all, 600.0, 1000).unwrap();
    assert!(g.starts_with("G21\nG90\nF600\nG0 X"));
    assert_eq!(g.matches("M3 S1000").count(), 1);
    assert_eq!(g.matches("M5").count(), 1);
    assert!(g.ends_with("M5\nM2\n"));
    assert!(
        g.find("M3").unwrap() > g.find("G0").unwrap(),
        "it is at the start before the laser goes on"
    );
    for bad in [0.0, -1.0, f64::NAN, 40_000.0] {
        assert!(gcode(&all, bad, 1000).is_err());
    }
}

#[test]
fn the_hpgl_has_forty_units_to_the_millimetre() {
    let all = paths(
        &[rect(5.0, 5.0, 1.0, 1.0)],
        &Profile {
            kerf_mm: 0.0,
            shrink_percent: 0.0,
            ..profile()
        },
    )
    .unwrap();
    let h = hpgl(&all);
    assert!(h.starts_with("IN;SP1;\nPU") && h.ends_with("PU0,0;SP0;\n"));
    assert!(
        h.contains("PD") && h.contains("220,218") && h.contains("180,182"),
        "the right edge is at 5.5 mm = 220 units and the corners start 0.05 mm inside the edges: {h}"
    );
}

#[test]
fn the_materials_have_profiles() {
    assert_eq!(Material::parse(" Mylar ").unwrap(), Material::Mylar);
    assert_eq!(Material::parse("polyimide").unwrap(), Material::Kapton);
    assert!(Material::parse("wood").unwrap_err().contains("wood"));
    for m in [Material::Mylar, Material::Kapton, Material::Steel] {
        Profile::of(m).validate().unwrap();
    }
    assert!(Profile::of(Material::Kapton).thickness_mm < Profile::of(Material::Mylar).thickness_mm);
}
