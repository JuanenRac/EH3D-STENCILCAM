//! The closed paths of the openings and the files that carry them to a laser (G-code) or to a plotter (HP-GL).

use crate::openings::{adjusted, Form, Opening};
use crate::profile::Profile;

/// Segments of a quarter of a circle.
const QUARTER: usize = 8;

pub type Path = Vec<(f64, f64)>;

/// The closed outline of an opening (the first point is repeated at the end), with corners of `radius` mm (a round or an obround opening is all corner).
pub fn outline(o: &Opening, radius: f64) -> Path {
    let r = match o.form {
        Form::Round | Form::Obround => o.w.min(o.h) / 2.0,
        Form::Rectangle => radius.max(0.0).min(o.w.min(o.h) / 2.0),
    };
    let (hw, hh) = (o.w / 2.0, o.h / 2.0);
    // the centres of the four corner arcs, counter-clockwise from the upper right one
    let corners = [
        (hw - r, hh - r, 0.0),
        (-hw + r, hh - r, 90.0),
        (-hw + r, -hh + r, 180.0),
        (hw - r, -hh + r, 270.0),
    ];
    let mut path = Vec::new();
    for (cx, cy, start) in corners {
        for k in 0..=QUARTER {
            let a = (start + 90.0 * k as f64 / QUARTER as f64).to_radians();
            path.push((o.x + cx + r * a.cos(), o.y + cy + r * a.sin()));
        }
    }
    path.push(path[0]);
    path
}

/// Every opening of the layer as a path to cut, adjusted for the material. A layer where an opening disappears is an error (nothing is cut half way).
pub fn paths(openings: &[Opening], p: &Profile) -> Result<Vec<Path>, String> {
    openings
        .iter()
        .map(|o| adjusted(o, p).map(|a| outline(&a, p.corner_radius_mm)))
        .collect()
}

/// The paths in the order of a greedy walk: always the nearest one that is left, starting at the origin, so that the head does not cross the sheet back and forth.
pub fn ordered(paths: &[Path]) -> Vec<Path> {
    let mut left: Vec<&Path> = paths.iter().collect();
    let mut at = (0.0, 0.0);
    let mut out = Vec::with_capacity(paths.len());
    while !left.is_empty() {
        let d = |p: &&Path| ((p[0].0 - at.0).powi(2) + (p[0].1 - at.1).powi(2)).sqrt();
        let i = left
            .iter()
            .enumerate()
            .min_by(|a, b| {
                d(a.1)
                    .partial_cmp(&d(b.1))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        let next = left.remove(i);
        at = next[0];
        out.push(next.clone());
    }
    out
}

fn n(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// G-code for a laser at constant power (`M3`): travel with the laser off, cut each path at `feed_mm_min`.
pub fn gcode(paths: &[Path], feed_mm_min: f64, s_value: u32) -> Result<String, String> {
    if !(feed_mm_min.is_finite() && feed_mm_min > 0.0 && feed_mm_min <= 30_000.0) {
        return Err(format!(
            "the feed is {feed_mm_min}: it must be from 0 to 30000 mm/min"
        ));
    }
    let mut g = format!("G21\nG90\nF{}\n", n(feed_mm_min));
    for path in ordered(paths) {
        g.push_str(&format!(
            "G0 X{} Y{}\nM3 S{s_value}\n",
            n(path[0].0),
            n(path[0].1)
        ));
        for p in &path[1..] {
            g.push_str(&format!("G1 X{} Y{}\n", n(p.0), n(p.1)));
        }
        g.push_str("M5\n");
    }
    g.push_str("M2\n");
    Ok(g)
}

/// HP-GL for a plotter or a cutter: 40 units to the millimetre, pen 1 down while a path is drawn.
pub fn hpgl(paths: &[Path]) -> String {
    let u = |v: f64| (v * 40.0).round() as i64;
    let mut s = String::from("IN;SP1;\n");
    for path in ordered(paths) {
        s.push_str(&format!("PU{},{};\nPD", u(path[0].0), u(path[0].1)));
        s.push_str(
            &path[1..]
                .iter()
                .map(|p| format!("{},{}", u(p.0), u(p.1)))
                .collect::<Vec<_>>()
                .join(","),
        );
        s.push_str(";\n");
    }
    s.push_str("PU0,0;SP0;\n");
    s
}
