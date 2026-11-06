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
    gcode_with_leads(paths, feed_mm_min, s_value, wait_s, 0.0, 0.0)
}

/// The most a lead-in or a lead-out may measure, in millimetres.
pub const MAX_LEAD_MM: f64 = 5.0;

/// Points closer than this (in millimetres) are the same point: the end of a closed circle comes back to its start with a rounding error of the sine.
const SAME_POINT_MM: f64 = 1e-9;

/// Whether two points are the same point.
fn same_point(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) < SAME_POINT_MM
}

/// The unit vector of the first segment of a path that has a length (a path may start with repeated points), or `None` when every point is the same.
fn first_direction(path: &[(f64, f64)]) -> Option<(f64, f64)> {
    let a = *path.first()?;
    path.iter()
        .skip(1)
        .find(|p| !same_point(**p, a))
        .map(|b| unit(a, *b))
}

/// The unit vector of the last segment of a path that has a length, pointing into the last point.
fn last_direction(path: &[(f64, f64)]) -> Option<(f64, f64)> {
    let z = *path.last()?;
    path.iter()
        .rev()
        .skip(1)
        .find(|p| !same_point(**p, z))
        .map(|a| unit(*a, z))
}

fn unit(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx.hypot(dy);
    (dx / len, dy / len)
}

/// The point where the tool enters the sheet and the point where it leaves it (each is `None` when its lead is 0).
pub type LeadPoints = (Option<(f64, f64)>, Option<(f64, f64)>);

/// Where the head enters and where it leaves a closed path: the point `lead_in_mm` before its first point on the straight line of its first segment, and the point
/// `lead_out_mm` after its last point on the straight line of its last segment (the lead lines are the prolongations of the contour, tangent to it, so the laser does not
/// turn at the join). An outline of the stencil is convex and runs counter-clockwise, so both points lie outside the opening, on the sheet. A length of 0, or a path with no
/// length at all, gives `None` for that end.
pub fn lead_points(path: &[(f64, f64)], lead_in_mm: f64, lead_out_mm: f64) -> LeadPoints {
    let mut entry = None;
    let mut exit = None;
    if let (true, Some(u)) = (lead_in_mm > 0.0, first_direction(path)) {
        entry = Some((path[0].0 - u.0 * lead_in_mm, path[0].1 - u.1 * lead_in_mm));
    }
    if let (true, Some(v)) = (lead_out_mm > 0.0, last_direction(path)) {
        let z = path[path.len() - 1];
        exit = Some((z.0 + v.0 * lead_out_mm, z.1 + v.1 * lead_out_mm));
    }
    (entry, exit)
}

/// Like [`gcode_with_wait`], with a straight tangent lead-in of `lead_in_mm` before each path and a lead-out of `lead_out_mm` after it (see [`lead_points`]; each at most
/// [`MAX_LEAD_MM`]). The head travels with the laser off to the start of the lead-in, the laser is switched on there (and waits, when asked) and cuts the lead-in, the path
/// and the lead-out, and is switched off at the end of the lead-out. With both leads at 0 the program is the one of [`gcode_with_wait`].
pub fn gcode_with_leads(
    paths: &[Path],
    feed_mm_min: f64,
    s_value: u32,
    wait_s: f64,
    lead_in_mm: f64,
    lead_out_mm: f64,
) -> Result<String, String> {
    if !(wait_s.is_finite() && (0.0..=60.0).contains(&wait_s)) {
        return Err(format!(
            "the laser wait is {wait_s}: it must be from 0 to 60 seconds"
        ));
    }
    for (name, v) in [("lead-in", lead_in_mm), ("lead-out", lead_out_mm)] {
        if !(v.is_finite() && (0.0..=MAX_LEAD_MM).contains(&v)) {
            return Err(format!(
                "the {name} is {v}: it must be from 0 to {MAX_LEAD_MM} mm"
            ));
        }
    }
    if !(feed_mm_min.is_finite() && feed_mm_min > 0.0 && feed_mm_min <= 30_000.0) {
        return Err(format!(
            "the feed is {feed_mm_min}: it must be from 0 to 30000 mm/min"
        ));
    }
    let mut g = format!(
        "G21
G90
F{}
",
        number(feed_mm_min)?
    );
    for path in ordered(paths) {
        let (entry, exit) = lead_points(&path, lead_in_mm, lead_out_mm);
        let start = entry.unwrap_or(path[0]);
        g.push_str(&format!(
            "G0 X{} Y{}
M3 S{s_value}
",
            number(start.0)?,
            number(start.1)?
        ));
        if wait_s > 0.0 {
            g.push_str(&format!(
                "G4 P{}
",
                number(wait_s)?
            ));
        }
        if entry.is_some() {
            g.push_str(&format!(
                "G1 X{} Y{}
",
                number(path[0].0)?,
                number(path[0].1)?
            ));
        }
        for p in &path[1..] {
            g.push_str(&format!(
                "G1 X{} Y{}
",
                number(p.0)?,
                number(p.1)?
            ));
        }
        if let Some(e) = exit {
            g.push_str(&format!(
                "G1 X{} Y{}
",
                number(e.0)?,
                number(e.1)?
            ));
        }
        g.push_str(
            "M5
",
        );
    }
    g.push_str(
        "M2
",
    );
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
