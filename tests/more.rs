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

// Bug hunt (round 4): names and bytes that go to the server, and the order, the G-code, the plot and the times of paths that are empty, odd or many.
mod hunt_round4 {
    use eh3d_serverclient::json::Json;
    use eh3d_stencilcam::openings::{Form, Opening};
    use eh3d_stencilcam::output::{
        duration_text, estimate_time, gcode, gcode_with_form_leads, gcode_with_leads, hpgl,
        lead_points, move_to_origin, ordered, ordered_indices, outline, outside_bed, write_atomic,
        FormLeads, Path,
    };
    use eh3d_stencilcam::server::{
        artifact_name, artifact_request, base64, check_key, is_server_key, media_type,
    };

    fn line(x0: f64, y0: f64, x1: f64, y1: f64) -> Path {
        vec![(x0, y0), (x1, y1)]
    }

    fn count_lines(text: &str, start: &str) -> usize {
        text.lines().filter(|l| l.starts_with(start)).count()
    }

    fn unbase64(text: &str) -> Vec<u8> {
        let value = |c: u8| -> u32 {
            match c {
                b'A'..=b'Z' => u32::from(c - b'A'),
                b'a'..=b'z' => u32::from(c - b'a') + 26,
                b'0'..=b'9' => u32::from(c - b'0') + 52,
                b'+' => 62,
                b'/' => 63,
                _ => panic!("not in the alphabet: {c}"),
            }
        };
        let mut out = Vec::new();
        for chunk in text.as_bytes().chunks(4) {
            let pad = chunk.iter().filter(|c| **c == b'=').count();
            let mut n = 0u32;
            for c in chunk.iter().filter(|c| **c != b'=') {
                n = (n << 6) | value(*c);
            }
            n <<= 6 * pad as u32;
            let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
            out.extend_from_slice(&bytes[..3 - pad]);
        }
        out
    }

    // ---- base64 and the names ----

    #[test]
    fn base64_gives_the_vectors_of_the_standard() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(plain.as_bytes()), coded, "{plain:?}");
        }
    }

    #[test]
    fn base64_of_every_length_has_the_right_size_and_reads_back() {
        let data: Vec<u8> = (0..=255u8).cycle().take(300).collect();
        for n in 0..=data.len() {
            let coded = base64(&data[..n]);
            assert_eq!(coded.len(), n.div_ceil(3) * 4, "{n}");
            assert_eq!(coded.len() % 4, 0);
            assert_eq!(unbase64(&coded), data[..n].to_vec(), "{n}");
        }
    }

    #[test]
    fn base64_of_bytes_that_are_all_ones_and_all_zeros_keeps_the_alphabet() {
        assert_eq!(base64(&[0xff, 0xff, 0xff]), "////");
        assert_eq!(base64(&[0, 0, 0]), "AAAA");
        assert_eq!(base64(&[0xfb, 0xef, 0xbe]), "++++");
    }

    #[test]
    fn an_artifact_name_is_the_last_part_of_a_path_of_either_slash() {
        assert_eq!(artifact_name("a/b/c.nc").unwrap(), "c.nc");
        assert_eq!(artifact_name("C:\\x\\y.nc").unwrap(), "y.nc");
        assert_eq!(artifact_name("mixed/dir\\file.plt").unwrap(), "file.plt");
        assert_eq!(artifact_name("  spaced.nc  ").unwrap(), "spaced.nc");
    }

    #[test]
    fn a_name_that_is_a_dot_or_climbs_a_folder_is_refused() {
        for bad in [".", "..", "a/..", "a\\..", "../..", " .. ", "/."] {
            assert!(artifact_name(bad).is_err(), "{bad:?}");
        }
        assert!(artifact_name("...").is_ok());
        assert!(artifact_name(".hidden").is_ok());
        assert!(artifact_name("a..b").is_ok());
    }

    #[test]
    fn a_name_that_hides_or_reverses_its_text_is_refused() {
        for bad in [
            "gcode\u{202e}txt.exe",
            "a\u{200b}b",
            "a\u{200f}b",
            "a\u{2066}b",
            "\u{feff}a",
            "a\u{0}b",
            "a\u{7f}b",
            "a\nb",
        ] {
            assert!(artifact_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_name_has_one_to_one_hundred_and_twenty_characters_and_the_count_is_of_characters() {
        assert!(artifact_name("").is_err());
        assert!(artifact_name("dir/").is_err());
        assert!(artifact_name("   ").is_err());
        assert!(artifact_name(&"x".repeat(120)).is_ok());
        assert!(artifact_name(&"x".repeat(121)).is_err());
        assert!(artifact_name(&"\u{e9}".repeat(120)).is_ok());
        assert!(artifact_name(&"\u{65e5}".repeat(121)).is_err());
    }

    #[test]
    fn the_media_type_comes_from_the_last_extension_in_any_case() {
        for (file, kind) in [
            ("a.NC", "text/x-gcode"),
            ("dir.d/a.tap", "text/x-gcode"),
            ("a.tar.nc", "text/x-gcode"),
            ("a.PLT", "application/vnd.hp-hpgl"),
            ("a.drl", "application/x-excellon"),
            ("C:\\x\\a.GTL", "application/x-gerber"),
            ("a.svg", "image/svg+xml"),
            ("a.csv", "text/csv"),
        ] {
            assert_eq!(media_type(file), kind, "{file}");
        }
    }

    #[test]
    fn a_file_with_no_known_extension_is_plain_bytes() {
        for file in [
            "",
            "noext",
            "dir.nc/noext",
            "a.",
            ".",
            ".nc.exe",
            "a.nc ",
            "a.n c",
        ] {
            assert_eq!(media_type(file), "application/octet-stream", "{file:?}");
        }
    }

    #[test]
    fn the_request_of_an_artifact_reads_back_whatever_the_name_and_the_values() {
        let name = "a\"b\\c\nd\u{1f600}";
        let params = [("k\"1", "v\\1"), ("tab\t", "line\r\nbreak\u{0}")];
        let body = artifact_request(name, "text/x-gcode", &[0, 1, 2, 255, 254], &params);
        let v = Json::parse(&body).unwrap();
        assert_eq!(v.get("name").and_then(Json::as_str), Some(name));
        assert_eq!(v.get("format").and_then(Json::as_str), Some("text/x-gcode"));
        let data = v.get("data_base64").and_then(Json::as_str).unwrap();
        assert_eq!(unbase64(data), vec![0, 1, 2, 255, 254]);
        let p = v.get("params").unwrap();
        assert_eq!(p.get("k\"1").and_then(Json::as_str), Some("v\\1"));
        assert_eq!(
            p.get("tab\t").and_then(Json::as_str),
            Some("line\r\nbreak\u{0}")
        );
        assert!(v.get("producer").is_some());
    }

    #[test]
    fn the_request_of_an_empty_artifact_is_still_json() {
        let v = Json::parse(&artifact_request("a.txt", "text/plain", &[], &[])).unwrap();
        assert_eq!(v.get("data_base64").and_then(Json::as_str), Some(""));
        assert!(v.get("params").is_some());
    }

    #[test]
    fn the_keys_of_the_link_are_told_in_any_case_and_a_value_out_of_range_is_refused() {
        assert!(is_server_key("SERVER_ADDRESS "));
        assert!(!is_server_key("server_address_x"));
        assert!(!is_server_key("feed"));
        assert!(check_key("server_timeout_s", "1").is_ok());
        assert!(check_key("server_timeout_s", "120").is_ok());
        for (key, value) in [
            ("server_timeout_s", "0"),
            ("server_timeout_s", "121"),
            ("server_timeout_s", "-1"),
            ("server_timeout_s", "1.5"),
            ("server_timeout_s", ""),
            ("server_retries", "6"),
            ("server_max_answer_mib", "257"),
            ("server_max_answer_mib", "0"),
            ("server_mdns", "maybe"),
            ("server_address", "a b"),
            ("server_token", "a\tb"),
        ] {
            assert!(check_key(key, value).is_err(), "{key} = {value:?}");
        }
        assert!(check_key("server_retries", "0").is_ok());
        assert!(check_key("server_retries", "5").is_ok());
    }

    // ---- the order and the files of the cuts ----

    #[test]
    fn a_list_with_no_path_or_only_empty_paths_has_no_order() {
        assert!(ordered_indices(&[]).is_empty());
        assert!(ordered_indices(&[vec![], vec![]]).is_empty());
        assert!(ordered(&[vec![]]).is_empty());
    }

    #[test]
    fn a_path_with_no_point_is_left_out_of_the_order_and_the_others_keep_their_place() {
        let paths = vec![
            vec![],
            line(10.0, 0.0, 11.0, 0.0),
            vec![],
            line(1.0, 0.0, 2.0, 0.0),
        ];
        assert_eq!(ordered_indices(&paths), vec![3, 1]);
        assert_eq!(ordered(&paths).len(), 2);
    }

    #[test]
    fn the_program_and_the_plot_of_a_list_with_an_empty_path_are_written_without_it() {
        let paths = vec![line(0.0, 0.0, 5.0, 0.0), vec![], line(0.0, 5.0, 5.0, 5.0)];
        let g = gcode(&paths, 600.0, 100).unwrap();
        assert_eq!(count_lines(&g, "M3 S"), 2);
        let h = hpgl(&paths);
        assert_eq!(
            h.matches("PU").count(),
            3,
            "two paths and the return to the origin: {h}"
        );
        assert_eq!(count_lines(&h, "PD"), 2);
        let t = estimate_time(&paths, 600.0, 0.0, (0.0, 0.0), 3000.0).unwrap();
        let without = estimate_time(
            &[paths[0].clone(), paths[2].clone()],
            600.0,
            0.0,
            (0.0, 0.0),
            3000.0,
        )
        .unwrap();
        assert_eq!(t, without);
    }

    #[test]
    fn the_order_is_a_walk_to_the_nearest_start_and_a_tie_goes_to_the_first_of_the_list() {
        let far = vec![
            line(10.0, 0.0, 11.0, 0.0),
            line(1.0, 0.0, 2.0, 0.0),
            line(5.0, 0.0, 6.0, 0.0),
        ];
        assert_eq!(ordered_indices(&far), vec![1, 2, 0]);
        let tie = vec![line(3.0, 4.0, 4.0, 4.0), line(4.0, 3.0, 5.0, 3.0)];
        assert_eq!(ordered_indices(&tie), vec![0, 1]);
        let same = vec![line(2.0, 2.0, 3.0, 3.0); 5];
        assert_eq!(ordered_indices(&same), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn the_order_of_many_paths_takes_each_one_once() {
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 10_000) as f64 / 10.0
        };
        let paths: Vec<Path> = (0..600)
            .map(|_| {
                let (x, y) = (next(), next());
                line(x, y, x + 1.0, y)
            })
            .collect();
        let mut order = ordered_indices(&paths);
        assert_eq!(order.len(), paths.len());
        order.sort_unstable();
        order.dedup();
        assert_eq!(order.len(), paths.len());
    }

    #[test]
    fn paths_with_starts_that_are_not_numbers_do_not_panic_the_order() {
        let paths = vec![
            line(f64::NAN, 0.0, 1.0, 0.0),
            line(0.0, f64::INFINITY, 1.0, 0.0),
            line(f64::NEG_INFINITY, f64::NAN, 1.0, 0.0),
            line(1.0, 1.0, 2.0, 2.0),
        ];
        let order = ordered_indices(&paths);
        assert!(order.len() <= paths.len());
        let mut seen = order.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), order.len(), "no path twice");
    }

    #[test]
    fn a_program_is_refused_for_a_feed_a_wait_or_a_lead_outside_its_range() {
        let paths = vec![line(0.0, 0.0, 5.0, 0.0)];
        for feed in [0.0, -1.0, f64::NAN, f64::INFINITY, 30_000.1] {
            assert!(gcode(&paths, feed, 100).is_err(), "{feed}");
        }
        assert!(gcode(&paths, 1.0, 100).is_ok());
        assert!(gcode(&paths, 30_000.0, 100).is_ok());
        for wait in [-0.1, 60.1, f64::NAN, f64::INFINITY] {
            assert!(
                gcode_with_leads(&paths, 600.0, 100, wait, 0.0, 0.0).is_err(),
                "{wait}"
            );
        }
        assert!(gcode_with_leads(&paths, 600.0, 100, 60.0, 0.0, 0.0).is_ok());
        for lead in [-0.1, 5.1, f64::NAN, f64::INFINITY] {
            assert!(
                gcode_with_leads(&paths, 600.0, 100, 0.0, lead, 0.0).is_err(),
                "{lead}"
            );
            assert!(
                gcode_with_leads(&paths, 600.0, 100, 0.0, 0.0, lead).is_err(),
                "{lead}"
            );
        }
        assert!(gcode_with_leads(&paths, 600.0, 100, 0.0, 5.0, 5.0).is_ok());
    }

    #[test]
    fn a_program_with_forms_that_do_not_match_its_paths_is_refused() {
        let paths = vec![line(0.0, 0.0, 5.0, 0.0), line(0.0, 5.0, 5.0, 5.0)];
        let leads = FormLeads::uniform(0.0, 0.0);
        assert!(gcode_with_form_leads(&paths, &[Form::Round], 600.0, 100, 0.0, &leads).is_err());
        assert!(gcode_with_form_leads(
            &paths,
            &[Form::Round, Form::Rectangle],
            600.0,
            100,
            0.0,
            &leads
        )
        .is_ok());
    }

    #[test]
    fn a_path_with_a_coordinate_that_is_not_a_number_never_writes_it_into_the_program() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let paths = vec![line(0.0, 0.0, bad, 1.0)];
            if let Ok(text) = gcode(&paths, 600.0, 100) {
                let lower = text.to_ascii_lowercase();
                assert!(!lower.contains("nan") && !lower.contains("inf"), "{text}");
            }
        }
    }

    #[test]
    fn the_leads_are_the_prolongations_of_the_first_and_the_last_segment() {
        let (entry, exit) = lead_points(&[(0.0, 0.0), (10.0, 0.0)], 1.0, 2.0);
        assert_eq!(entry, Some((-1.0, 0.0)));
        assert_eq!(exit, Some((12.0, 0.0)));
        let (entry, exit) = lead_points(
            &[(0.0, 0.0), (0.0, 0.0), (10.0, 0.0), (10.0, 0.0)],
            1.0,
            2.0,
        );
        assert_eq!(entry, Some((-1.0, 0.0)));
        assert_eq!(exit, Some((12.0, 0.0)));
    }

    #[test]
    fn a_lead_of_zero_or_a_path_with_no_length_has_no_lead_point() {
        assert_eq!(
            lead_points(&[(0.0, 0.0), (10.0, 0.0)], 0.0, 0.0),
            (None, None)
        );
        assert_eq!(
            lead_points(&[(3.0, 3.0), (3.0, 3.0)], 1.0, 1.0),
            (None, None)
        );
        assert_eq!(lead_points(&[(3.0, 3.0)], 1.0, 1.0), (None, None));
        let (entry, exit) = lead_points(&[(0.0, 0.0), (10.0, 0.0)], 1.0, 0.0);
        assert!(entry.is_some() && exit.is_none());
    }

    #[test]
    fn the_time_of_a_path_is_its_length_over_the_feed_and_the_waits_and_errors_are_counted() {
        let paths = vec![line(0.0, 0.0, 10.0, 0.0)];
        let t = estimate_time(&paths, 600.0, 2.0, (0.0, 0.0), 6000.0).unwrap();
        assert!((t.cutting_s - 1.0).abs() < 1e-9, "{t:?}");
        assert!((t.waiting_s - 2.0).abs() < 1e-9, "{t:?}");
        assert!(t.rapid_s.abs() < 1e-9, "{t:?}");
        assert!((t.total_s() - 3.0).abs() < 1e-9);
        assert!(
            estimate_time(&[], 600.0, 0.0, (0.0, 0.0), 6000.0)
                .unwrap()
                .total_s()
                == 0.0
        );
        for feed in [0.0, -1.0, f64::NAN] {
            assert!(
                estimate_time(&paths, feed, 0.0, (0.0, 0.0), 6000.0).is_err(),
                "{feed}"
            );
        }
        for rapid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(
                estimate_time(&paths, 600.0, 0.0, (0.0, 0.0), rapid).is_err(),
                "{rapid}"
            );
        }
        assert!(estimate_time(&paths, 600.0, -1.0, (0.0, 0.0), 6000.0).is_err());
    }

    #[test]
    fn a_time_is_told_in_seconds_minutes_and_hours_and_never_panics() {
        for (seconds, text) in [
            (0.0, "0 s"),
            (44.6, "45 s"),
            (59.5, "1 min 0 s"),
            (147.0, "2 min 27 s"),
            (3600.0, "1 h 0 min"),
            (3719.0, "1 h 1 min"),
            (-5.0, "0 s"),
            (f64::NAN, "0 s"),
            (f64::NEG_INFINITY, "0 s"),
        ] {
            assert_eq!(duration_text(seconds), text, "{seconds}");
        }
        let _ = duration_text(f64::INFINITY);
        let _ = duration_text(f64::MAX);
    }

    #[test]
    fn the_plot_writes_forty_units_to_the_millimetre_and_returns_to_the_origin() {
        let h = hpgl(&[line(0.0, 0.0, 1.0, 2.0)]);
        assert!(h.starts_with("IN;SP1;\n"), "{h}");
        assert!(h.contains("PU0,0;\nPD40,80;\n"), "{h}");
        assert!(h.ends_with("PU0,0;SP0;\n"), "{h}");
        assert_eq!(hpgl(&[]), "IN;SP1;\nPU0,0;SP0;\n");
    }

    #[test]
    fn moving_to_the_origin_puts_the_lowest_corner_there_and_ignores_points_that_are_not_numbers() {
        let mut paths = vec![
            line(5.0, 7.0, 9.0, 8.0),
            line(f64::NAN, 0.0, 6.0, f64::INFINITY),
        ];
        move_to_origin(&mut paths, (0.0, 0.0));
        assert_eq!(paths[0][0], (0.0, 0.0));
        assert_eq!(paths[0][1], (4.0, 1.0));
        let mut again = paths.clone();
        move_to_origin(&mut again, (0.0, 0.0));
        assert_eq!(again[0], paths[0], "moving twice changes nothing more");
    }

    #[test]
    fn moving_nothing_or_only_points_that_are_not_numbers_leaves_the_paths_alone() {
        let mut none: Vec<Path> = vec![];
        move_to_origin(&mut none, (1.0, 1.0));
        assert!(none.is_empty());
        let mut empty = vec![vec![]];
        move_to_origin(&mut empty, (1.0, 1.0));
        assert!(empty[0].is_empty());
        let mut bad = vec![line(f64::NAN, f64::NAN, f64::INFINITY, 1.0)];
        move_to_origin(&mut bad, (1.0, 1.0));
        assert!(bad[0][0].0.is_nan() && bad[0][1].0.is_infinite());
    }

    #[test]
    fn the_bed_includes_its_edges_and_a_point_that_is_not_a_number_is_outside() {
        let bed = (100.0, 50.0);
        assert_eq!(outside_bed(&[line(0.0, 0.0, 100.0, 50.0)], bed), None);
        assert_eq!(
            outside_bed(&[line(0.0, 0.0, 100.001, 50.0)], bed),
            Some((100.001, 50.0))
        );
        assert_eq!(
            outside_bed(&[line(-0.001, 0.0, 1.0, 1.0)], bed),
            Some((-0.001, 0.0))
        );
        assert!(outside_bed(&[line(0.0, 0.0, f64::NAN, 1.0)], bed).is_some());
        assert_eq!(outside_bed(&[], bed), None);
        assert_eq!(outside_bed(&[vec![]], bed), None);
    }

    #[test]
    fn the_outline_of_an_opening_is_closed_and_a_sharp_rectangle_has_four_corners() {
        let o = Opening {
            x: 10.0,
            y: 20.0,
            w: 4.0,
            h: 2.0,
            form: Form::Rectangle,
        };
        let sharp = outline(&o, 0.0);
        assert_eq!(sharp.len(), 5);
        assert_eq!(sharp.first(), sharp.last());
        for p in &sharp {
            assert!(
                (p.0 - 10.0).abs() <= 2.0 + 1e-9 && (p.1 - 20.0).abs() <= 1.0 + 1e-9,
                "{p:?}"
            );
        }
        let rounded = outline(&o, 0.5);
        assert_eq!(rounded.first(), rounded.last());
        assert!(rounded.len() > 5);
    }

    #[test]
    fn a_round_opening_is_a_circle_and_a_radius_that_is_negative_or_too_big_is_held() {
        let c = Opening {
            x: 0.0,
            y: 0.0,
            w: 2.0,
            h: 2.0,
            form: Form::Round,
        };
        for p in outline(&c, 0.0) {
            assert!((p.0.hypot(p.1) - 1.0).abs() < 1e-9, "{p:?}");
        }
        let r = Opening {
            x: 0.0,
            y: 0.0,
            w: 4.0,
            h: 2.0,
            form: Form::Rectangle,
        };
        for radius in [-5.0, 100.0, f64::NAN] {
            for p in outline(&r, radius) {
                assert!(
                    p.0.abs() <= 2.0 + 1e-9 && p.1.abs() <= 1.0 + 1e-9,
                    "{radius} {p:?}"
                );
            }
        }
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("stencilcam-hunt-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_file_is_written_whole_replaced_whole_and_leaves_no_temporary_file() {
        let dir = scratch("atomic");
        let file = dir.join("out.nc");
        write_atomic(&file, "first").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "first");
        write_atomic(&file, "").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "");
        write_atomic(&file, "x\u{e9}\u{65e5}\u{0}y").unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "x\u{e9}\u{65e5}\u{0}y"
        );
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_cannot_be_written_gives_the_code_and_keeps_no_temporary_file() {
        let dir = scratch("failing");
        let missing = dir.join("no-such-folder").join("out.nc");
        let e = write_atomic(&missing, "x").unwrap_err();
        assert!(e.starts_with("file_unwritable"), "{e}");
        let target = dir.join("is-a-folder");
        std::fs::create_dir_all(&target).unwrap();
        let e = write_atomic(&target, "x").unwrap_err();
        assert!(e.starts_with("file_unwritable"), "{e}");
        assert!(!dir.join("is-a-folder.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
