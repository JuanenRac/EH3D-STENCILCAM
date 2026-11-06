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

/// The forms of the openings, in the order of [`paths`] (the form of each path, to choose its leads).
pub fn forms(openings: &[Opening]) -> Vec<Form> {
    openings.iter().map(|o| o.form).collect()
}

/// The paths in the order of a greedy walk: always the nearest one that is left, starting at the origin, so that the head does not cross the sheet back and forth.
/// The start points go in a grid of about one per cell and the nearest one is looked for ring by ring, so a layer of a hundred thousand openings is ordered in moments
/// (the plain walk is quadratic). The result is the one of the plain walk: of two paths at the same distance the first of the list goes first.
pub fn ordered(paths: &[Path]) -> Vec<Path> {
    ordered_indices(paths)
        .into_iter()
        .map(|k| paths[k].clone())
        .collect()
}

/// The indices of `paths` in the order of [`ordered`], so that what belongs to each path (its form, for example) can follow it.
pub fn ordered_indices(paths: &[Path]) -> Vec<usize> {
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
        out.push(k);
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

/// The length of the lead-in and of the lead-out of a cut, in millimetres (0: none).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lead {
    pub in_mm: f64,
    pub out_mm: f64,
}

/// The leads of the cuts by the form of the opening: one for a round opening, one for a rectangular one and one for the rest (the obround openings and every path whose form is not
/// given).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormLeads {
    pub general: Lead,
    pub round: Lead,
    pub rect: Lead,
}

impl FormLeads {
    /// The same leads for every form (what [`gcode_with_leads`] writes).
    pub fn uniform(in_mm: f64, out_mm: f64) -> FormLeads {
        let lead = Lead { in_mm, out_mm };
        FormLeads {
            general: lead,
            round: lead,
            rect: lead,
        }
    }

    /// The leads of an opening of the form `form`: a circle uses `round`, a rectangle `rect`, an obround opening `general`.
    pub fn for_form(&self, form: Form) -> Lead {
        match form {
            Form::Round => self.round,
            Form::Rectangle => self.rect,
            Form::Obround => self.general,
        }
    }

    /// Every length with the name the messages call it by, in the order they are checked.
    pub fn named(&self) -> [(&'static str, f64); 6] {
        [
            ("lead-in", self.general.in_mm),
            ("lead-out", self.general.out_mm),
            ("lead-in-round", self.round.in_mm),
            ("lead-out-round", self.round.out_mm),
            ("lead-in-rect", self.rect.in_mm),
            ("lead-out-rect", self.rect.out_mm),
        ]
    }
}

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
    write_gcode(
        paths,
        None,
        feed_mm_min,
        s_value,
        wait_s,
        &FormLeads::uniform(lead_in_mm, lead_out_mm),
    )
}

/// Like [`gcode_with_leads`], with the leads chosen by the form of each opening (`forms[i]` is the form of `paths[i]`; see [`FormLeads::for_form`]). Both lists must have the same length.
pub fn gcode_with_form_leads(
    paths: &[Path],
    forms: &[Form],
    feed_mm_min: f64,
    s_value: u32,
    wait_s: f64,
    leads: &FormLeads,
) -> Result<String, String> {
    if forms.len() != paths.len() {
        return Err(format!(
            "there are {} forms for {} paths",
            forms.len(),
            paths.len()
        ));
    }
    write_gcode(paths, Some(forms), feed_mm_min, s_value, wait_s, leads)
}

fn write_gcode(
    paths: &[Path],
    forms: Option<&[Form]>,
    feed_mm_min: f64,
    s_value: u32,
    wait_s: f64,
    leads: &FormLeads,
) -> Result<String, String> {
    if !(wait_s.is_finite() && (0.0..=60.0).contains(&wait_s)) {
        return Err(format!(
            "the laser wait is {wait_s}: it must be from 0 to 60 seconds"
        ));
    }
    for (name, v) in leads.named() {
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
    for k in ordered_indices(paths) {
        let path = &paths[k];
        let lead = forms.map_or(leads.general, |f| leads.for_form(f[k]));
        let (entry, exit) = lead_points(path, lead.in_mm, lead.out_mm);
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

/// The time of a G-code job, estimated from the speeds and not from a machine: the seconds of cutting, of waiting for the laser and of the moves with the laser off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeEstimate {
    pub cutting_s: f64,
    pub waiting_s: f64,
    pub rapid_s: f64,
}

impl TimeEstimate {
    pub fn total_s(&self) -> f64 {
        self.cutting_s + self.waiting_s + self.rapid_s
    }
}

/// The time [`gcode_with_leads`] would take for the same arguments plus `rapid_mm_min`, the speed of the moves with the laser off. The head starts at the origin and walks
/// the paths in the order of [`ordered`]: it travels to the start of each lead-in (or of the path), cuts the lead-in, the path and the lead-out at `feed_mm_min`, and waits
/// `wait_s` after each `M3`. Acceleration is not counted, so the real job takes longer. A feed or a rapid speed that is not above zero is refused.
pub fn estimate_time(
    paths: &[Path],
    feed_mm_min: f64,
    wait_s: f64,
    leads: (f64, f64),
    rapid_mm_min: f64,
) -> Result<TimeEstimate, String> {
    estimate_inner(
        paths,
        None,
        feed_mm_min,
        wait_s,
        &FormLeads::uniform(leads.0, leads.1),
        rapid_mm_min,
    )
}

/// Like [`estimate_time`], with the leads chosen by the form of each opening, the way [`gcode_with_form_leads`] writes them (`forms[i]` is the form of `paths[i]`).
pub fn estimate_time_with_forms(
    paths: &[Path],
    forms: &[Form],
    feed_mm_min: f64,
    wait_s: f64,
    leads: &FormLeads,
    rapid_mm_min: f64,
) -> Result<TimeEstimate, String> {
    if forms.len() != paths.len() {
        return Err(format!(
            "there are {} forms for {} paths",
            forms.len(),
            paths.len()
        ));
    }
    estimate_inner(paths, Some(forms), feed_mm_min, wait_s, leads, rapid_mm_min)
}

fn estimate_inner(
    paths: &[Path],
    forms: Option<&[Form]>,
    feed_mm_min: f64,
    wait_s: f64,
    leads: &FormLeads,
    rapid_mm_min: f64,
) -> Result<TimeEstimate, String> {
    if !(feed_mm_min.is_finite() && feed_mm_min > 0.0) {
        return Err("the feed must be above 0 to estimate the time".into());
    }
    if !(rapid_mm_min.is_finite() && rapid_mm_min > 0.0) {
        return Err("the rapid speed must be above 0 to estimate the time".into());
    }
    if !(wait_s.is_finite() && wait_s >= 0.0) {
        return Err("the laser wait must not be negative to estimate the time".into());
    }
    let (mut cut_mm, mut travel_mm, mut runs) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut at = (0.0_f64, 0.0_f64);
    for k in ordered_indices(paths) {
        let path = &paths[k];
        let lead = forms.map_or(leads.general, |f| leads.for_form(f[k]));
        let (entry, exit) = lead_points(path, lead.in_mm, lead.out_mm);
        let start = entry.unwrap_or(path[0]);
        travel_mm += (start.0 - at.0).hypot(start.1 - at.1);
        if let Some(e) = entry {
            cut_mm += (path[0].0 - e.0).hypot(path[0].1 - e.1);
        }
        cut_mm += path
            .windows(2)
            .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
            .sum::<f64>();
        let last = path[path.len() - 1];
        at = last;
        if let Some(x) = exit {
            cut_mm += (x.0 - last.0).hypot(x.1 - last.1);
            at = x;
        }
        runs += 1.0;
    }
    Ok(TimeEstimate {
        cutting_s: cut_mm / feed_mm_min * 60.0,
        waiting_s: runs * wait_s,
        rapid_s: travel_mm / rapid_mm_min * 60.0,
    })
}

/// A time for a person: `45 s`, `2 min 27 s`, `1 h 2 min` (the seconds are rounded first, and the seconds are left out from an hour on).
pub fn duration_text(seconds: f64) -> String {
    let total = if seconds.is_finite() && seconds > 0.0 {
        seconds.round() as u64
    } else {
        0
    };
    let (h, m, s) = (total / 3600, total % 3600 / 60, total % 60);
    if h > 0 {
        format!("{h} h {m} min")
    } else if m > 0 {
        format!("{m} min {s} s")
    } else {
        format!("{s} s")
    }
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
