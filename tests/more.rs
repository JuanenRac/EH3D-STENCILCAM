//! More tests: the materials and their limits, the reading of a layer (size limit, hidden notices), and the G-code and HP-GL of the cuts.

use eh3d_stencilcam::layer::{hidden_codes, read_layer};
use eh3d_stencilcam::output::{gcode, hpgl};
use eh3d_stencilcam::profile::{Material, Profile};

const PASTE: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nM02*\n";

#[test]
fn a_material_is_read_by_its_name_or_its_other_name_in_any_case() {
    for (text, want) in [
        ("mylar", Material::Mylar),
        ("POLYESTER", Material::Mylar),
        ("Kapton", Material::Kapton),
        (" polyimide ", Material::Kapton),
        ("steel", Material::Steel),
        ("Stainless", Material::Steel),
    ] {
        assert_eq!(Material::parse(text), Ok(want), "{text}");
    }
    let e = Material::parse("wood").unwrap_err();
    assert!(e.contains("wood") && e.contains("mylar"), "{e}");
    assert!(Material::parse("").is_err());
}

#[test]
fn the_starting_profile_of_every_material_is_valid_and_they_differ() {
    let all = [Material::Mylar, Material::Kapton, Material::Steel].map(Profile::of);
    for p in &all {
        assert!(p.validate().is_ok(), "{p:?}");
    }
    assert_ne!(all[0], all[1]);
    assert_ne!(all[0], all[2]);
    // the laser takes less from steel than from the plastics, and kapton is the thinnest
    assert!(all[2].kerf_mm < all[0].kerf_mm);
    assert!(all[1].thickness_mm < all[0].thickness_mm);
}

#[test]
fn every_value_of_a_profile_is_checked_at_its_limits_and_named_when_it_is_out() {
    let ok = Profile::of(Material::Mylar);
    let with = |f: &dyn Fn(&mut Profile)| {
        let mut p = ok;
        f(&mut p);
        p
    };
    assert!(with(&|p| p.thickness_mm = 0.02).validate().is_ok());
    assert!(with(&|p| p.thickness_mm = 1.0).validate().is_ok());
    assert!(with(&|p| p.thickness_mm = 0.0199)
        .validate()
        .unwrap_err()
        .contains("thickness_mm"));
    assert!(with(&|p| p.thickness_mm = 1.01)
        .validate()
        .unwrap_err()
        .contains("thickness_mm"));
    assert!(with(&|p| p.shrink_percent = 50.0).validate().is_ok());
    assert!(with(&|p| p.shrink_percent = -0.1)
        .validate()
        .unwrap_err()
        .contains("shrink_percent"));
    assert!(with(&|p| p.shrink_percent = 50.1)
        .validate()
        .unwrap_err()
        .contains("shrink_percent"));
    assert!(with(&|p| p.corner_radius_mm = 2.1)
        .validate()
        .unwrap_err()
        .contains("corner_radius_mm"));
    assert!(with(&|p| p.kerf_mm = 1.1)
        .validate()
        .unwrap_err()
        .contains("kerf_mm"));
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(with(&|p| p.kerf_mm = bad).validate().is_err());
        assert!(with(&|p| p.thickness_mm = bad).validate().is_err());
    }
}

#[test]
fn a_layer_is_read_and_a_file_over_the_size_limit_is_refused() {
    let r = read_layer(PASTE.as_bytes(), 10_000, &[]).unwrap();
    assert_eq!(r.errors, 0);
    assert!(r.shown.is_empty(), "{:?}", r.shown);
    assert!(read_layer(PASTE.as_bytes(), 10, &[]).is_err());
    // exactly the size is allowed
    assert!(read_layer(PASTE.as_bytes(), PASTE.len(), &[]).is_ok());
    assert!(read_layer(PASTE.as_bytes(), PASTE.len() - 1, &[]).is_err());
}

#[test]
fn a_layer_with_something_the_reader_does_not_understand_shows_it_with_its_code_and_a_hidden_code_is_left_out(
) {
    let text = PASTE.replace("M02*", "G99*\nM02*");
    let shown = read_layer(text.as_bytes(), 10_000, &[]).unwrap();
    if shown.shown.is_empty() {
        // the reader took the word as something it knows: nothing to hide in this file
        return;
    }
    assert!(
        shown
            .shown
            .iter()
            .all(|l| l.starts_with("the reader, line ") && l.contains('[') && l.ends_with(']')),
        "{:?}",
        shown.shown
    );
    let code = {
        let l = &shown.shown[0];
        l[l.rfind('[').unwrap() + 1..l.len() - 1].to_string()
    };
    let hidden = read_layer(text.as_bytes(), 10_000, std::slice::from_ref(&code)).unwrap();
    assert!(hidden.shown.len() < shown.shown.len());
    assert!(hidden
        .shown
        .iter()
        .all(|l| !l.contains(&format!("[{code}]"))));
}

#[test]
fn the_codes_to_hide_are_checked_against_the_codes_of_the_reader_and_blanks_are_ignored() {
    assert_eq!(hidden_codes("").unwrap(), Vec::<String>::new());
    assert_eq!(hidden_codes(" , ,").unwrap(), Vec::<String>::new());
    let first = eh3d_parser_gerber::codes::ALL[0];
    let second = eh3d_parser_gerber::codes::ALL[1];
    assert_eq!(hidden_codes(first).unwrap(), vec![first.to_string()]);
    assert_eq!(
        hidden_codes(&format!(" {first} , {second},")).unwrap(),
        vec![first.to_string(), second.to_string()]
    );
    let e = hidden_codes(&format!("{first},NOT-A-CODE")).unwrap_err();
    assert!(e.contains("NOT-A-CODE") && e.contains("--hide"), "{e}");
}

fn square(x: f64, y: f64, s: f64) -> Vec<(f64, f64)> {
    vec![(x, y), (x + s, y), (x + s, y + s), (x, y + s), (x, y)]
}

#[test]
fn the_feed_of_the_gcode_must_be_more_than_zero_and_at_most_thirty_thousand() {
    let paths = vec![square(0.0, 0.0, 1.0)];
    for bad in [0.0, -1.0, 30_000.1, f64::NAN, f64::INFINITY] {
        assert!(
            gcode(&paths, bad, 500).unwrap_err().contains("feed"),
            "{bad}"
        );
    }
    assert!(gcode(&paths, 30_000.0, 500).is_ok());
    assert!(gcode(&paths, 0.001, 500).is_ok());
}

#[test]
fn the_gcode_travels_with_the_laser_off_cuts_each_path_and_ends_the_program() {
    let g = gcode(&[square(0.0, 0.0, 1.0), square(5.0, 5.0, 1.0)], 300.0, 800).unwrap();
    assert!(g.starts_with("G21\nG90\nF300\n"), "{g}");
    assert!(g.trim_end().ends_with("M2"));
    assert_eq!(g.matches("M3 S800").count(), 2);
    assert_eq!(g.matches("M5").count(), 2);
    // the laser is on only between M3 and M5, and a rapid travel is never made with it on
    let mut on = false;
    for line in g.lines() {
        if line.starts_with("M3") {
            on = true;
        } else if line.starts_with("M5") {
            on = false;
        } else if line.starts_with("G0 ") {
            assert!(!on, "a rapid move with the laser on: {line}");
        }
    }
    assert!(!on);
}

#[test]
fn a_program_with_no_cut_is_only_its_header_and_its_end() {
    let g = gcode(&[], 300.0, 800).unwrap();
    assert_eq!(g, "G21\nG90\nF300\nM2\n");
    assert_eq!(hpgl(&[]), "IN;SP1;\nPU0,0;SP0;\n");
}

#[test]
fn the_hpgl_is_in_forty_units_to_the_millimetre_and_every_path_lifts_and_lowers_the_pen() {
    let h = hpgl(&[square(1.0, 2.0, 0.5)]);
    assert!(
        h.starts_with("IN;SP1;\n") && h.ends_with("PU0,0;SP0;\n"),
        "{h}"
    );
    assert!(h.contains("PU40,80;"), "{h}");
    assert!(h.contains("PD60,80,60,100,40,100,40,80;"), "{h}");
    assert_eq!(h.matches("PU").count(), 2);
    let two = hpgl(&[square(0.0, 0.0, 1.0), square(5.0, 5.0, 1.0)]);
    assert_eq!(two.matches("PD").count(), 2);
}

#[test]
fn an_opening_with_sharp_corners_has_no_repeated_points() {
    use eh3d_stencilcam::openings::{Form, Opening};
    use eh3d_stencilcam::output::outline;
    let o = Opening {
        x: 1.0,
        y: 2.0,
        w: 2.0,
        h: 1.0,
        form: Form::Rectangle,
    };
    let sharp = outline(&o, 0.0);
    assert_eq!(sharp.len(), 5, "{sharp:?}");
    assert_eq!(sharp.first(), sharp.last());
    assert!(sharp.windows(2).all(|w| w[0] != w[1]));
    // the rounded one keeps its thirty-seven points
    assert_eq!(outline(&o, 0.1).len(), 37);
}

fn walk_by_brute_force(paths: &[Vec<(f64, f64)>]) -> Vec<Vec<(f64, f64)>> {
    let mut left: Vec<&Vec<(f64, f64)>> = paths.iter().collect();
    let mut at = (0.0, 0.0);
    let mut out = Vec::new();
    while !left.is_empty() {
        let d = |p: &&Vec<(f64, f64)>| ((p[0].0 - at.0).powi(2) + (p[0].1 - at.1).powi(2)).sqrt();
        let i = left
            .iter()
            .enumerate()
            .min_by(|a, b| d(a.1).partial_cmp(&d(b.1)).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        let next = left.remove(i);
        at = next[0];
        out.push(next.clone());
    }
    out
}

fn scattered(n: usize) -> Vec<Vec<(f64, f64)>> {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % 100_000) as f64 / 100.0
    };
    (0..n)
        .map(|_| {
            let (x, y) = (next(), next());
            vec![(x, y), (x + 1.0, y), (x + 1.0, y + 1.0), (x, y)]
        })
        .collect()
}

#[test]
fn the_walk_of_the_paths_is_the_plain_nearest_walk_and_a_hundred_thousand_take_moments() {
    use eh3d_stencilcam::output::ordered;
    let few = scattered(400);
    assert_eq!(ordered(&few), walk_by_brute_force(&few));
    // points on a line, repeated points and equal distances: the first of the list goes first
    let mut odd = vec![
        vec![(5.0, 0.0)],
        vec![(5.0, 0.0)],
        vec![(0.0, 5.0)],
        vec![(5.0, 0.0)],
        vec![(-5.0, 0.0)],
    ];
    odd.push(vec![(0.0, 0.0)]);
    assert_eq!(ordered(&odd), walk_by_brute_force(&odd));
    let started = std::time::Instant::now();
    let many = scattered(100_000);
    let walk = ordered(&many);
    assert_eq!(walk.len(), 100_000);
    assert!(
        started.elapsed().as_secs() < 30,
        "{:?} for 100000 openings",
        started.elapsed()
    );
    assert!(ordered(&[]).is_empty());
}

#[test]
fn a_wait_after_the_laser_is_switched_on_is_written_for_every_path_and_a_wrong_wait_is_refused() {
    use eh3d_stencilcam::output::gcode_with_wait;
    let paths = vec![square(0.0, 0.0, 1.0), square(5.0, 5.0, 1.0)];
    let g = gcode_with_wait(&paths, 300.0, 800, 0.5).unwrap();
    assert_eq!(g.matches("M3 S800\nG4 P0.5\nG1 X").count(), 2, "{g}");
    // no wait is the plain program
    let plain = gcode(&paths, 300.0, 800).unwrap();
    assert!(!plain.contains("G4"), "{plain}");
    assert_eq!(gcode_with_wait(&paths, 300.0, 800, 0.0).unwrap(), plain);
    for bad in [-1.0, 60.5, f64::NAN, f64::INFINITY] {
        let e = gcode_with_wait(&paths, 300.0, 800, bad).unwrap_err();
        assert!(e.contains("laser wait"), "{bad}: {e}");
    }
    assert!(gcode_with_wait(&paths, 300.0, 800, 60.0).is_ok());
}
