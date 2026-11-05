//! The closed paths of the openings and the files that carry them to a laser (G-code) or to a plotter (HP-GL).

use crate::openings::{adjusted, Form, Opening};
use crate::profile::Profile;
use eh3d_parser_gcode::emit::number;

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
    if r == 0.0 {
        // sharp corners: every corner arc is the same point repeated, and the laser would stop there for nothing
        path.dedup();
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
/// The start points go in a grid of about one per cell and the nearest one is looked for ring by ring, so a layer of a hundred thousand openings is ordered in moments
/// (the plain walk is quadratic). The result is the one of the plain walk: of two paths at the same distance the first of the list goes first.
pub fn ordered(paths: &[Path]) -> Vec<Path> {
    let n = paths.len();
    let mut out = Vec::with_capacity(n);
    if n == 0 {
        return out;
    }
    let starts: Vec<(f64, f64)> = paths.iter().map(|p| p[0]).collect();
    let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
    let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for s in &starts {
        x0 = x0.min(s.0);
        y0 = y0.min(s.1);
        x1 = x1.max(s.0);
        y1 = y1.max(s.1);
    }
    let side = ((n as f64).sqrt().ceil() as usize).clamp(1, 2048);
    let cw = ((x1 - x0) / side as f64).max(1e-9);
    let ch = ((y1 - y0) / side as f64).max(1e-9);
    let cell = |p: (f64, f64)| -> (usize, usize) {
        let cx = (((p.0 - x0) / cw).floor().max(0.0) as usize).min(side - 1);
        let cy = (((p.1 - y0) / ch).floor().max(0.0) as usize).min(side - 1);
        (cx, cy)
    };
    let mut grid: Vec<Vec<usize>> = vec![Vec::new(); side * side];
    for (k, s) in starts.iter().enumerate() {
        let (cx, cy) = cell(*s);
        grid[cy * side + cx].push(k);
    }
    let smallest_cell = cw.min(ch);
    let mut at = (0.0, 0.0);
    for _ in 0..n {
        let (cx, cy) = cell(at);
        let mut best: Option<(f64, usize)> = None;
        for r in 0..=side as i64 {
            for dx in -r..=r {
                let on_edge = dx.abs() == r;
                let mut dy = -r;
                while dy <= r {
                    let (gx, gy) = (cx as i64 + dx, cy as i64 + dy);
                    if gx >= 0 && gy >= 0 && (gx as usize) < side && (gy as usize) < side {
                        for &k in &grid[gy as usize * side + gx as usize] {
                            let d = ((starts[k].0 - at.0).powi(2) + (starts[k].1 - at.1).powi(2))
                                .sqrt();
                            let better = match best {
                                None => true,
                                Some((bd, bk)) => d < bd || (d == bd && k < bk),
                            };
                            if better {
                                best = Some((d, k));
                            }
                        }
                    }
                    // inside the ring only its two sides are visited, on its top and bottom rows every cell
                    dy += if on_edge || r == 0 { 1 } else { 2 * r };
                }
            }
            if let Some((bd, _)) = best {
                if bd < r as f64 * smallest_cell {
                    break;
                }
            }
        }
        let Some((_, k)) = best else { break };
        let (kx, ky) = cell(starts[k]);
        let bucket = &mut grid[ky * side + kx];
        if let Some(pos) = bucket.iter().position(|&m| m == k) {
            bucket.swap_remove(pos);
        }
        at = starts[k];
        out.push(paths[k].clone());
    }
    out
}

/// G-code for a laser at constant power (`M3`): travel with the laser off, cut each path at `feed_mm_min`.
pub fn gcode(paths: &[Path], feed_mm_min: f64, s_value: u32) -> Result<String, String> {
    gcode_with_wait(paths, feed_mm_min, s_value, 0.0)
}

/// Like [`gcode`], and after each `M3` a wait of `wait_s` seconds (`G4 P`) so that the laser is at its power before the head moves; 0 writes no wait. At most 60 seconds.
pub fn gcode_with_wait(
    paths: &[Path],
    feed_mm_min: f64,
    s_value: u32,
    wait_s: f64,
) -> Result<String, String> {
    if !(wait_s.is_finite() && (0.0..=60.0).contains(&wait_s)) {
        return Err(format!(
            "the laser wait is {wait_s}: it must be from 0 to 60 seconds"
        ));
    }
    if !(feed_mm_min.is_finite() && feed_mm_min > 0.0 && feed_mm_min <= 30_000.0) {
        return Err(format!(
            "the feed is {feed_mm_min}: it must be from 0 to 30000 mm/min"
        ));
    }
    let mut g = format!("G21\nG90\nF{}\n", number(feed_mm_min)?);
    for path in ordered(paths) {
        g.push_str(&format!(
            "G0 X{} Y{}\nM3 S{s_value}\n",
            number(path[0].0)?,
            number(path[0].1)?
        ));
        if wait_s > 0.0 {
            g.push_str(&format!("G4 P{}\n", number(wait_s)?));
        }
        for p in &path[1..] {
            g.push_str(&format!("G1 X{} Y{}\n", number(p.0)?, number(p.1)?));
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
