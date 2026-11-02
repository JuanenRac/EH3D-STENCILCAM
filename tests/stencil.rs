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

// ---- more edges: exact thresholds, symmetries of the outline, formatting and the walk of the paths ----

fn disc(x: f64, y: f64, d: f64) -> Opening {
    Opening {
        x,
        y,
        w: d,
        h: d,
        form: Form::Round,
    }
}

fn area(p: &[(f64, f64)]) -> f64 {
    // signed area of a closed path (the first point repeated at the end)
    p.windows(2)
        .map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1)
        .sum::<f64>()
        / 2.0
}

#[test]
fn the_precise_ratios_of_a_round_and_a_square_opening_agree() {
    let r = ratios(&disc(0.0, 0.0, 0.4), 0.1).unwrap();
    assert!(
        (r.aspect - 4.0).abs() < 1e-9 && (r.area - 1.0).abs() < 1e-9,
        "{r:?}"
    );
    // a square of the same side as the diameter of the circle has the same area ratio: s / (4 t)
    let q = ratios(&rect(0.0, 0.0, 0.4, 0.4), 0.1).unwrap();
    assert!((q.area - r.area).abs() < 1e-9);
}

#[test]
fn the_limits_of_the_aspect_and_of_the_area_are_exact_and_only_one_reason_is_given() {
    let t = 0.1;
    // aspect exactly 1.5 passes the aspect test; its area ratio 0.375 does not pass the area test
    let weak_by_area = weak(&[rect(0.0, 0.0, 0.1875, 0.1875)], 0.125).unwrap();
    assert_eq!(weak_by_area.len(), 1);
    assert!(
        weak_by_area[0].1.contains("area ratio"),
        "{}",
        weak_by_area[0].1
    );
    // just under 1.5 is weak by the aspect, and the message does not mention the area
    let under = weak(&[rect(0.0, 0.0, 0.1499, 0.1499)], t).unwrap();
    assert!(under[0].1.contains("aspect ratio") && !under[0].1.contains("area ratio"));
    // a square passes when s / (4 t) reaches 0.66: 0.27 does, 0.26 does not
    assert!(weak(&[rect(0.0, 0.0, 0.27, 0.27)], t).unwrap().is_empty());
    assert_eq!(weak(&[rect(0.0, 0.0, 0.26, 0.26)], t).unwrap().len(), 1);
    // the first opening is number 1 in the message and 0 in the index
    let two = weak(&[rect(0.0, 0.0, 5.0, 5.0), rect(9.0, 0.0, 0.1, 0.1)], t).unwrap();
    assert_eq!(two[0].0, 1);
    assert!(two[0].1.starts_with("opening 2 "), "{}", two[0].1);
}

#[test]
fn a_profile_is_valid_exactly_inside_its_ranges() {
    let ok = |p: Profile| p.validate().is_ok();
    assert!(
        ok(Profile {
            thickness_mm: 0.02,
            ..profile()
        }) && !ok(Profile {
            thickness_mm: 0.0199,
            ..profile()
        })
    );
    assert!(
        ok(Profile {
            thickness_mm: 1.0,
            ..profile()
        }) && !ok(Profile {
            thickness_mm: 1.0001,
            ..profile()
        })
    );
    assert!(
        ok(Profile {
            shrink_percent: 0.0,
            ..profile()
        }) && ok(Profile {
            shrink_percent: 50.0,
            ..profile()
        })
    );
    assert!(
        !ok(Profile {
            shrink_percent: -0.1,
            ..profile()
        }) && !ok(Profile {
            shrink_percent: f64::INFINITY,
            ..profile()
        })
    );
    assert!(
        ok(Profile {
            corner_radius_mm: 2.0,
            ..profile()
        }) && !ok(Profile {
            corner_radius_mm: 2.01,
            ..profile()
        })
    );
    assert!(
        ok(Profile {
            kerf_mm: 0.0,
            ..profile()
        }) && ok(Profile {
            kerf_mm: 1.0,
            ..profile()
        }) && !ok(Profile {
            kerf_mm: 1.01,
            ..profile()
        })
    );
    let e = Profile {
        kerf_mm: 7.0,
        ..profile()
    }
    .validate()
    .unwrap_err();
    assert!(e.contains("kerf_mm") && e.contains('7'), "{e}");
}

#[test]
fn a_shrink_of_nothing_keeps_the_size_and_half_halves_it_and_the_centre_never_moves() {
    let o = rect(3.0, 4.0, 1.2, 0.8);
    let same = shrunk(
        &o,
        &Profile {
            shrink_percent: 0.0,
            ..profile()
        },
    )
    .unwrap();
    assert_eq!(same, o);
    let half = shrunk(
        &o,
        &Profile {
            shrink_percent: 50.0,
            ..profile()
        },
    )
    .unwrap();
    assert!(
        (half.w - 0.6).abs() < 1e-12
            && (half.h - 0.4).abs() < 1e-12
            && (half.x, half.y) == (3.0, 4.0)
    );
}

#[test]
fn the_kerf_that_eats_the_whole_opening_is_the_error_and_the_one_just_under_is_not() {
    let p = Profile {
        shrink_percent: 0.0,
        ..profile()
    };
    let o = rect(0.0, 0.0, 0.5, 0.5);
    assert!(
        adjusted(&o, &Profile { kerf_mm: 0.5, ..p }).is_err(),
        "nothing left is an error, not a zero-size path"
    );
    let a = adjusted(&o, &Profile { kerf_mm: 0.49, ..p }).unwrap();
    assert!((a.w - 0.01).abs() < 1e-9);
    assert_eq!(adjusted(&o, &Profile { kerf_mm: 0.0, ..p }).unwrap(), o);
}

#[test]
fn an_outline_has_thirty_seven_points_runs_counter_clockwise_and_stays_in_the_opening() {
    for o in [
        rect(10.0, 20.0, 2.0, 1.0),
        disc(-3.0, 4.0, 0.8),
        Opening {
            x: 0.0,
            y: 0.0,
            w: 3.0,
            h: 1.0,
            form: Form::Obround,
        },
        Opening {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 3.0,
            form: Form::Obround,
        },
    ] {
        let p = outline(&o, 0.1);
        assert_eq!(p.len(), 37, "{o:?}");
        assert_eq!(p.first(), p.last());
        assert!(area(&p) > 0.0, "{o:?} runs clockwise");
        assert!(
            p.iter()
                .all(|q| (q.0 - o.x).abs() <= o.w / 2.0 + 1e-9
                    && (q.1 - o.y).abs() <= o.h / 2.0 + 1e-9),
            "{o:?}"
        );
        assert!(area(&p) <= o.w * o.h + 1e-9);
    }
}

#[test]
fn a_rounded_corner_takes_the_expected_area_off_a_rectangle() {
    let sharp = area(&outline(&rect(0.0, 0.0, 4.0, 2.0), 0.0));
    assert!((sharp - 8.0).abs() < 1e-9);
    let rounded = area(&outline(&rect(0.0, 0.0, 4.0, 2.0), 0.5));
    // four corners lose (1 - pi/4) r^2 each; the polygon of eight segments a quarter is a little smaller still
    let ideal = 8.0 - 4.0 * (1.0 - std::f64::consts::FRAC_PI_4) * 0.25;
    assert!(
        rounded < sharp && (rounded - ideal).abs() < 0.01,
        "{rounded} against {ideal}"
    );
    // a negative radius is a sharp corner
    assert_eq!(
        outline(&rect(0.0, 0.0, 4.0, 2.0), -3.0),
        outline(&rect(0.0, 0.0, 4.0, 2.0), 0.0)
    );
}

#[test]
fn an_obround_opening_has_semicircular_ends_whatever_its_direction() {
    let wide = outline(
        &Opening {
            x: 0.0,
            y: 0.0,
            w: 4.0,
            h: 1.0,
            form: Form::Obround,
        },
        0.0,
    );
    // the area of a stadium: (w - h) * h + pi * (h / 2)^2
    let want = 3.0 + std::f64::consts::PI * 0.25;
    assert!((area(&wide) - want).abs() < 0.01, "{}", area(&wide));
    let tall = outline(
        &Opening {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 4.0,
            form: Form::Obround,
        },
        0.0,
    );
    assert!((area(&tall) - area(&wide)).abs() < 1e-9);
}

#[test]
fn the_walk_visits_every_path_once_whatever_the_order_it_was_given() {
    let list: Vec<_> = [
        (30.0, 5.0),
        (2.0, 40.0),
        (15.0, 15.0),
        (15.0, 15.5),
        (80.0, 1.0),
        (0.5, 0.5),
    ]
    .iter()
    .map(|&(x, y)| outline(&rect(x, y, 1.0, 1.0), 0.0))
    .collect();
    let mut reversed = list.clone();
    reversed.reverse();
    let a = ordered(&list);
    let b = ordered(&reversed);
    assert_eq!(a.len(), list.len());
    let key = |p: &Vec<(f64, f64)>| format!("{:?}", p[0]);
    let mut seen: Vec<String> = a.iter().map(key).collect();
    seen.sort();
    let mut want: Vec<String> = list.iter().map(key).collect();
    want.sort();
    assert_eq!(seen, want);
    assert_eq!(
        a.iter().map(key).collect::<Vec<_>>(),
        b.iter().map(key).collect::<Vec<_>>(),
        "no ties: the walk does not depend on the input order"
    );
    assert_eq!(ordered(&list), a, "the same input, the same walk");
}

#[test]
fn the_gcode_has_one_laser_burst_per_path_and_never_cuts_with_the_laser_off() {
    let all = paths(
        &[
            rect(5.0, 5.0, 1.0, 1.0),
            rect(20.0, 5.0, 1.0, 1.0),
            rect(40.0, 9.0, 0.8, 0.5),
        ],
        &profile(),
    )
    .unwrap();
    let g = gcode(&all, 750.5, 800).unwrap();
    assert!(g.contains("F750.5\n"));
    assert_eq!(g.matches("M3 S800").count(), 3);
    assert_eq!(g.matches("M5").count(), 3);
    assert_eq!(g.matches("G0 X").count(), 3);
    let mut on = false;
    for line in g.lines() {
        if line.starts_with("M3") {
            on = true;
        } else if line.starts_with("M5") {
            on = false;
        } else if line.starts_with("G1") {
            assert!(on, "a cut with the laser off: {line}");
        } else if line.starts_with("G0") {
            assert!(!on, "a rapid with the laser on: {line}");
        }
    }
    assert!(!on, "the laser is off at the end");
    assert_eq!(gcode(&[], 600.0, 1000).unwrap(), "G21\nG90\nF600\nM2\n");
}

#[test]
fn the_numbers_of_the_gcode_never_say_minus_zero_and_have_at_most_three_decimals() {
    let tiny = vec![(-0.0004, 0.0), (1.23456, -0.0001), (-0.0004, 0.0)];
    let g = gcode(&[tiny], 600.0, 1000).unwrap();
    assert!(
        !g.contains("-0 ") && !g.contains("-0\n") && !g.contains("X-0."),
        "{g}"
    );
    assert!(
        g.contains("G0 X0 Y0\n") && g.contains("G1 X1.235 Y0\n"),
        "{g}"
    );
}

#[test]
fn the_hpgl_of_nothing_is_just_the_frame_and_negative_coordinates_keep_their_sign() {
    assert_eq!(hpgl(&[]), "IN;SP1;\nPU0,0;SP0;\n");
    let h = hpgl(&[vec![(-1.0, -2.0), (0.0, 0.0), (-1.0, -2.0)]]);
    assert!(
        h.contains("PU-40,-80;") && h.contains("PD0,0,-40,-80;"),
        "{h}"
    );
}

#[test]
fn the_same_layer_gives_the_same_gcode_every_time() {
    let (o, _) = from_gerber(&eh3d_parser_gerber::parse(PASTE));
    let a = gcode(&paths(&o, &profile()).unwrap(), 600.0, 1000).unwrap();
    let b = gcode(&paths(&o, &profile()).unwrap(), 600.0, 1000).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        hpgl(&paths(&o, &profile()).unwrap()),
        hpgl(&paths(&o, &profile()).unwrap())
    );
}

#[test]
fn paths_stop_at_the_first_opening_that_disappears_and_name_it() {
    let e = paths(
        &[rect(0.0, 0.0, 1.0, 1.0), rect(7.0, 8.0, 0.05, 0.05)],
        &profile(),
    )
    .unwrap_err();
    assert!(e.contains("7.000") && e.contains("8.000"), "{e}");
}

#[test]
fn a_flash_of_zero_size_makes_no_opening_and_tracks_and_regions_are_noted_once() {
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0*%\n%ADD11C,0.5*%\nD10*\nX10000Y10000D03*\nD11*\nX20000Y20000D03*\nX0Y0D02*\nX10000Y0D01*\nX10000Y10000D01*\nG36*\nX0Y0D02*\nX10000Y0D01*\nX10000Y10000D01*\nG37*\nM02*\n";
    let (o, notes) = from_gerber(&eh3d_parser_gerber::parse(text));
    assert_eq!(o.len(), 1, "{o:?}");
    assert_eq!(
        notes
            .iter()
            .filter(|n| n.contains("tracks and regions"))
            .count(),
        1,
        "{notes:?}"
    );
}

#[test]
fn the_materials_are_known_by_their_aliases_in_any_case() {
    for (text, want) in [
        ("MYLAR", Material::Mylar),
        ("Polyester", Material::Mylar),
        (" kapton\t", Material::Kapton),
        ("POLYIMIDE", Material::Kapton),
        ("Steel", Material::Steel),
        ("stainless", Material::Steel),
    ] {
        assert_eq!(Material::parse(text).unwrap(), want, "{text:?}");
    }
    for bad in ["", "copper", "my lar"] {
        assert!(Material::parse(bad).is_err(), "{bad:?}");
    }
}
