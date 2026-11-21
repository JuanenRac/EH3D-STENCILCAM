//! The openings of the stencil: one for every pad that the paste layer flashes.

use crate::profile::Profile;
use eh3d_gerberviewer::scene::{self, Shape};
use eh3d_parser_gerber::GerberFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Rectangle,
    Round,
    Obround,
}

/// An opening centred at `(x, y)`, `w` wide and `h` high, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opening {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub form: Form,
}

/// The openings of a paste layer and what could not be turned into one (with the reason, once each). The shapes come from the same conversion of apertures that the Gerber viewer
/// of the suite uses: a circle, a rectangle or an obround pad is an opening, a track along an axis and a region that is a rectangle are too (see [`from_gerber_rules`]), and anything that could not become one is a note with its reason.
pub fn from_gerber(file: &GerberFile) -> (Vec<Opening>, Vec<String>) {
    from_gerber_with(file, scene::DEFAULT_ARC_TOLERANCE_MM)
}

/// The same, with the largest distance, in mm, between an arc and the polygon drawn for it.
pub fn from_gerber_with(file: &GerberFile, arc_tolerance_mm: f64) -> (Vec<Opening>, Vec<String>) {
    let shaped = from_gerber_rules(file, arc_tolerance_mm, &ShapeRules::default());
    (shaped.openings, shaped.notes)
}

/// How the shapes that are not flashed pads become openings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeRules {
    /// A region (a pad drawn as an outline, such as a macro pad with rounded corners) that fills its enclosing rectangle by at least 100 minus this percent is opened as that
    /// rectangle. 0 opens only a region that is exactly a rectangle.
    pub region_fit_percent: f64,
}

/// The most that `region_fit_percent` may be.
pub const MAX_REGION_FIT_PERCENT: f64 = 20.0;

impl Default for ShapeRules {
    fn default() -> Self {
        ShapeRules {
            region_fit_percent: 0.0,
        }
    }
}

impl ShapeRules {
    pub fn validate(&self) -> Result<(), String> {
        if self.region_fit_percent.is_finite()
            && (0.0..=MAX_REGION_FIT_PERCENT).contains(&self.region_fit_percent)
        {
            Ok(())
        } else {
            Err(format!(
                "region_fit_percent is {}: it must be from 0 to {MAX_REGION_FIT_PERCENT}",
                self.region_fit_percent
            ))
        }
    }
}

/// What a paste layer gives: the openings, what could not become one (each with its reason, and the stencil would be incomplete) and what was approximated on purpose (the stencil is
/// complete, but a shape was changed, and the person is told).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shaped {
    pub openings: Vec<Opening>,
    pub notes: Vec<String>,
    pub approximated: Vec<String>,
}

/// The distance, in mm, under which two coordinates are the same line.
const AXIS_EPS_MM: f64 = 1e-6;
/// How many reasons are written one by one before the rest are counted.
const MAX_REASONS: usize = 5;

fn stroke_opening(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Result<Opening, String> {
    if ![x0, y0, x1, y1, w].iter().all(|v| v.is_finite()) || w <= 0.0 {
        return Err("a track with no usable width or place".into());
    }
    let (dx, dy) = ((x1 - x0).abs(), (y1 - y0).abs());
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    if dx <= AXIS_EPS_MM && dy <= AXIS_EPS_MM {
        Ok(Opening {
            x: cx,
            y: cy,
            w,
            h: w,
            form: Form::Round,
        })
    } else if dy <= AXIS_EPS_MM {
        Ok(Opening {
            x: cx,
            y: cy,
            w: dx + w,
            h: w,
            form: Form::Obround,
        })
    } else if dx <= AXIS_EPS_MM {
        Ok(Opening {
            x: cx,
            y: cy,
            w,
            h: dy + w,
            form: Form::Obround,
        })
    } else {
        Err(format!("a track at an angle, from ({x0:.3}, {y0:.3}) to ({x1:.3}, {y1:.3}), cannot be an opening: only tracks along the axes can"))
    }
}

/// A region as an opening: the rectangle it is, or (when the rules allow it) the enclosing rectangle of a region that nearly fills it; the second value says that it was approximated.
fn region_opening(pts: &[(f64, f64)], rules: &ShapeRules) -> Result<(Opening, bool), String> {
    if pts.iter().any(|p| !(p.0.is_finite() && p.1.is_finite())) {
        return Err("a region with a corner that is not a number".into());
    }
    let mut v: Vec<(f64, f64)> = pts.to_vec();
    if v.len() > 1 && v[0] == v[v.len() - 1] {
        v.pop();
    }
    if v.len() < 3 {
        return Err("a region with fewer than three corners".into());
    }
    let (mut x_min, mut x_max, mut y_min, mut y_max) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for p in &v {
        x_min = x_min.min(p.0);
        x_max = x_max.max(p.0);
        y_min = y_min.min(p.1);
        y_max = y_max.max(p.1);
    }
    let (bw, bh) = (x_max - x_min, y_max - y_min);
    if bw <= AXIS_EPS_MM || bh <= AXIS_EPS_MM {
        return Err("a region with no area".into());
    }
    let area = (0..v.len())
        .map(|i| {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        .abs()
        / 2.0;
    let rect = Opening {
        x: (x_min + x_max) / 2.0,
        y: (y_min + y_max) / 2.0,
        w: bw,
        h: bh,
        form: Form::Rectangle,
    };
    let axis_aligned = (0..v.len()).all(|i| {
        let (a, b) = (v[i], v[(i + 1) % v.len()]);
        (a.0 - b.0).abs() <= AXIS_EPS_MM || (a.1 - b.1).abs() <= AXIS_EPS_MM
    });
    let fill = area / (bw * bh) * 100.0;
    if v.len() == 4 && axis_aligned && fill >= 100.0 - 1e-6 {
        return Ok((rect, false));
    }
    if rules.region_fit_percent > 0.0 && fill >= 100.0 - rules.region_fit_percent {
        return Ok((rect, true));
    }
    Err(format!(
        "a region of {} corners at ({:.3}, {:.3}) is not a rectangle (it fills {:.1} % of its enclosing rectangle); region_fit_percent opens one that fills nearly all of it",
        v.len(),
        rect.x,
        rect.y,
        fill
    ))
}

/// The same as [`from_gerber_with`] with the rules for the shapes that are not flashed pads: a track along an axis is an obround opening (a round one when it is a point), a region that
/// is a rectangle is a rectangular one, and a region that nearly fills its enclosing rectangle may be one when `rules` say so. Every other track or region is a note with its reason.
pub fn from_gerber_rules(file: &GerberFile, arc_tolerance_mm: f64, rules: &ShapeRules) -> Shaped {
    let scene = scene::from_gerber_with(file, &scene::SceneOptions { arc_tolerance_mm });
    let (mut out, mut notes) = (Vec::new(), scene.notes.clone());
    let (mut reasons, mut approximated) = (Vec::new(), 0usize);
    for shape in &scene.shapes {
        match *shape {
            Shape::Disc { x, y, r } => out.push(Opening {
                x,
                y,
                w: r * 2.0,
                h: r * 2.0,
                form: Form::Round,
            }),
            Shape::Rect { x, y, w, h } => out.push(Opening {
                x,
                y,
                w,
                h,
                form: Form::Rectangle,
            }),
            Shape::Obround { x, y, w, h } => out.push(Opening {
                x,
                y,
                w,
                h,
                form: Form::Obround,
            }),
            Shape::Stroke { x0, y0, x1, y1, w } => match stroke_opening(x0, y0, x1, y1, w) {
                Ok(o) => out.push(o),
                Err(why) => reasons.push(why),
            },
            Shape::Region { ref pts } => match region_opening(pts, rules) {
                Ok((o, approx)) => {
                    approximated += usize::from(approx);
                    out.push(o);
                }
                Err(why) => reasons.push(why),
            },
        }
    }
    let total = reasons.len();
    notes.extend(reasons.into_iter().take(MAX_REASONS));
    if total > MAX_REASONS {
        notes.push(format!(
            "{} more tracks or regions of the paste layer cannot become openings",
            total - MAX_REASONS
        ));
    }
    let approximated = if approximated > 0 {
        vec![format!(
            "{approximated} region(s) were opened as their enclosing rectangle (region_fit_percent {})",
            rules.region_fit_percent
        )]
    } else {
        Vec::new()
    };
    out.retain(|o| o.w > 0.0 && o.h > 0.0);
    Shaped {
        openings: out,
        notes,
        approximated,
    }
}

/// The opening that the paste will see: the pad made smaller by `shrink_percent` of each side.
pub fn shrunk(o: &Opening, p: &Profile) -> Result<Opening, String> {
    p.validate()?;
    let f = 1.0 - p.shrink_percent / 100.0;
    Ok(Opening {
        w: o.w * f,
        h: o.h * f,
        ..*o
    })
}

/// The line the laser has to follow: the shrunk opening reduced by the kerf (the beam removes half a kerf on each side of the line, so the hole comes out the size that was asked).
pub fn adjusted(o: &Opening, p: &Profile) -> Result<Opening, String> {
    let s = shrunk(o, p)?;
    let (w, h) = (s.w - p.kerf_mm, s.h - p.kerf_mm);
    if w <= 0.0 || h <= 0.0 {
        return Err(format!("the opening of {:.3} x {:.3} mm at ({:.3}, {:.3}) is gone after the shrink and the kerf", o.w, o.h, o.x, o.y));
    }
    Ok(Opening { w, h, ..s })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ratios {
    /// The smaller side of the opening over the thickness of the sheet: 1.5 or more lets the paste leave the opening.
    pub aspect: f64,
    /// The area of the opening over the area of its walls: 0.66 or more.
    pub area: f64,
}

pub const MIN_ASPECT: f64 = 1.5;
pub const MIN_AREA: f64 = 0.66;
/// The range of the aspect ratio limit and of the area ratio limit that may be asked for.
pub const ASPECT_LIMIT_RANGE: (f64, f64) = (0.5, 10.0);
pub const AREA_LIMIT_RANGE: (f64, f64) = (0.1, 2.0);

/// The two limits under which an opening does not release the paste.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub min_aspect: f64,
    pub min_area: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            min_aspect: MIN_ASPECT,
            min_area: MIN_AREA,
        }
    }
}

impl Limits {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, (lo, hi): (f64, f64)| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("min_aspect", self.min_aspect, ASPECT_LIMIT_RANGE)?;
        range("min_area", self.min_area, AREA_LIMIT_RANGE)
    }
}

/// The thickest sheet, in mm, for which the opening still meets both limits: its smaller side over the aspect limit, and the area ratio of the form over the area limit.
pub fn max_thickness_mm(o: &Opening, limits: &Limits) -> f64 {
    let by_aspect = o.w.min(o.h) / limits.min_aspect;
    // area ratio = opening area / (perimeter x thickness)
    let (area, perimeter) = match o.form {
        Form::Round => (
            std::f64::consts::PI * (o.w / 2.0) * (o.w / 2.0),
            std::f64::consts::PI * o.w,
        ),
        _ => (o.w * o.h, 2.0 * (o.w + o.h)),
    };
    by_aspect.min(area / (perimeter * limits.min_area))
}

/// The two ratios of an opening for a sheet of `thickness_mm`, computed on the size that the paste will see (the opening as it is cut).
pub fn ratios(o: &Opening, thickness_mm: f64) -> Result<Ratios, String> {
    if !(thickness_mm.is_finite() && thickness_mm > 0.0) {
        return Err(format!(
            "thickness_mm is {thickness_mm}: it must be more than 0"
        ));
    }
    let (opening_area, wall_area) = match o.form {
        Form::Round => {
            let r = o.w / 2.0;
            (
                std::f64::consts::PI * r * r,
                2.0 * std::f64::consts::PI * r * thickness_mm,
            )
        }
        _ => (o.w * o.h, 2.0 * (o.w + o.h) * thickness_mm),
    };
    Ok(Ratios {
        aspect: o.w.min(o.h) / thickness_mm,
        area: opening_area / wall_area,
    })
}

/// The openings whose ratios are below the limits, with the reason.
pub fn weak(openings: &[Opening], thickness_mm: f64) -> Result<Vec<(usize, String)>, String> {
    weak_with(openings, thickness_mm, &Limits::default())
}

/// The same with the limits given: each reason names the ratio, its limit and the thickest sheet for which the opening would do.
pub fn weak_with(
    openings: &[Opening],
    thickness_mm: f64,
    limits: &Limits,
) -> Result<Vec<(usize, String)>, String> {
    limits.validate()?;
    let mut out = Vec::new();
    for (i, o) in openings.iter().enumerate() {
        let r = ratios(o, thickness_mm)?;
        let (name, value, limit) = if r.aspect < limits.min_aspect {
            ("aspect", r.aspect, limits.min_aspect)
        } else if r.area < limits.min_area {
            ("area", r.area, limits.min_area)
        } else {
            continue;
        };
        out.push((i, format!("opening {} ({:.3} x {:.3} mm at ({:.3}, {:.3})): {name} ratio {value:.2} is below {limit}; the paste will stay in it (a sheet up to {:.3} mm thick would do)", i + 1, o.w, o.h, o.x, o.y, max_thickness_mm(o, limits))));
    }
    Ok(out)
}

/// Mirrors the openings about the vertical line through the middle of the layer (a stencil for the bottom side is seen from the other face).
pub fn mirrored(openings: &[Opening]) -> Vec<Opening> {
    if openings.is_empty() {
        return Vec::new();
    }
    let min = openings
        .iter()
        .map(|o| o.x - o.w / 2.0)
        .fold(f64::INFINITY, f64::min);
    let max = openings
        .iter()
        .map(|o| o.x + o.w / 2.0)
        .fold(f64::NEG_INFINITY, f64::max);
    openings
        .iter()
        .map(|o| Opening {
            x: min + max - o.x,
            ..*o
        })
        .collect()
}

/// The openings with the ones that repeat another left out: two openings of the same form, size and place (to within `tolerance_mm`) are one hole, and cutting it twice would burn the
/// edge of the stencil. The first of each is kept, in its place; the second value is how many were left out.
pub fn deduplicated(openings: &[Opening], tolerance_mm: f64) -> (Vec<Opening>, usize) {
    let tol = if tolerance_mm.is_finite() && tolerance_mm > 0.0 {
        tolerance_mm
    } else {
        0.0
    };
    let same = |a: &Opening, b: &Opening| {
        a.form == b.form
            && (a.x - b.x).abs() <= tol
            && (a.y - b.y).abs() <= tol
            && (a.w - b.w).abs() <= tol
            && (a.h - b.h).abs() <= tol
    };
    // sorted by x, an opening is compared only with the ones whose x lies within the tolerance
    let mut order: Vec<usize> = (0..openings.len()).collect();
    order.sort_by(|&a, &b| openings[a].x.total_cmp(&openings[b].x).then(a.cmp(&b)));
    let mut drop = vec![false; openings.len()];
    for (k, &i) in order.iter().enumerate() {
        if drop[i] {
            continue;
        }
        for &j in &order[k + 1..] {
            if openings[j].x - openings[i].x > tol {
                break;
            }
            if !drop[j] && same(&openings[i], &openings[j]) {
                // the one that came first in the layer stays
                if j < i {
                    drop[i] = true;
                    break;
                }
                drop[j] = true;
            }
        }
    }
    let kept: Vec<Opening> = openings
        .iter()
        .zip(&drop)
        .filter(|(_, d)| !**d)
        .map(|(o, _)| *o)
        .collect();
    let removed = openings.len() - kept.len();
    (kept, removed)
}

/// The least distance, in mm, between the edges of two openings, as their enclosing rectangles see it (two round ones, by their circles); 0 or less when they touch or overlap.
pub fn gap(a: &Opening, b: &Opening) -> f64 {
    if a.form == Form::Round && b.form == Form::Round {
        return (a.x - b.x).hypot(a.y - b.y) - (a.w + b.w) / 2.0;
    }
    let dx = (a.x - b.x).abs() - (a.w + b.w) / 2.0;
    let dy = (a.y - b.y).abs() - (a.h + b.h) / 2.0;
    if dx <= 0.0 && dy <= 0.0 {
        dx.max(dy)
    } else {
        dx.max(0.0).hypot(dy.max(0.0))
    }
}

/// Two openings whose web (the sheet that stays between them) is narrower than the limit, by their numbers (from 1) and the width of the web.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NarrowWeb {
    pub first: usize,
    pub second: usize,
    pub web_mm: f64,
}

/// The pairs of openings whose web is narrower than `min_web_mm` (a negative web is two openings that overlap). 0 or less finds none, because every web is at least that wide.
/// The openings are the ones as they are cut. The pairs come in the order of the first opening, then the second.
pub fn narrow_webs(openings: &[Opening], min_web_mm: f64) -> Vec<NarrowWeb> {
    if !(min_web_mm.is_finite() && min_web_mm > 0.0) {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..openings.len()).collect();
    order.sort_by(|&a, &b| {
        (openings[a].x - openings[a].w / 2.0)
            .total_cmp(&(openings[b].x - openings[b].w / 2.0))
            .then(a.cmp(&b))
    });
    let mut out = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        let right = openings[i].x + openings[i].w / 2.0 + min_web_mm;
        for &j in &order[k + 1..] {
            if openings[j].x - openings[j].w / 2.0 > right {
                break;
            }
            let web = gap(&openings[i], &openings[j]);
            if web < min_web_mm {
                let (first, second) = (i.min(j), i.max(j));
                out.push(NarrowWeb {
                    first: first + 1,
                    second: second + 1,
                    web_mm: web,
                });
            }
        }
    }
    out.sort_by(|a, b| a.first.cmp(&b.first).then(a.second.cmp(&b.second)));
    out
}

/// The paste an opening holds when the sheet is `thickness_mm` thick, in cubic millimetres (the area of the opening as it is cut, times the thickness).
pub fn paste_volume_mm3(o: &Opening, thickness_mm: f64) -> f64 {
    let area = match o.form {
        Form::Round => std::f64::consts::PI * (o.w / 2.0) * (o.w / 2.0),
        _ => o.w * o.h,
    };
    area * thickness_mm
}

/// What a stencil needs in a sentence of numbers: how many openings of each form, and the paste (in microlitres, 1 mm3 = 1 uL) all of them hold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub round: usize,
    pub rectangle: usize,
    pub obround: usize,
    pub paste_ul: f64,
}

pub fn summary(openings: &[Opening], thickness_mm: f64) -> Summary {
    let count = |f: Form| openings.iter().filter(|o| o.form == f).count();
    Summary {
        round: count(Form::Round),
        rectangle: count(Form::Rectangle),
        obround: count(Form::Obround),
        paste_ul: openings
            .iter()
            .map(|o| paste_volume_mm3(o, thickness_mm))
            .sum(),
    }
}

/// A text as a JSON string, with its quotes: the quote, the backslash and the control characters are escaped.
pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_number(v: f64) -> String {
    if v.is_finite() {
        format!("{v}")
    } else {
        "null".into()
    }
}

/// What `check --json` prints: one JSON object with the number of openings by form, the paste they hold, the smallest ratios, the openings that will not release the paste, the
/// narrow webs and how many duplicate openings were left out. `shown` are the openings as the paste sees them (shrunk); the ratios are not given (`null`) when there are none.
pub fn check_json(
    shown: &[Opening],
    thickness_mm: f64,
    shrink_percent: f64,
    weak: &[(usize, String)],
    webs: &[NarrowWeb],
    duplicates_removed: usize,
) -> Result<String, String> {
    let all = shown
        .iter()
        .map(|o| ratios(o, thickness_mm))
        .collect::<Result<Vec<_>, _>>()?;
    let min_aspect = all.iter().map(|r| r.aspect).reduce(f64::min);
    let min_area = all.iter().map(|r| r.area).reduce(f64::min);
    let s = summary(shown, thickness_mm);
    let opt = |v: Option<f64>| v.map_or_else(|| "null".to_string(), json_number);
    let weak_items: Vec<String> = weak
        .iter()
        .map(|(i, m)| format!("{{\"opening\":{},\"message\":{}}}", i + 1, json_string(m)))
        .collect();
    let web_items: Vec<String> = webs
        .iter()
        .map(|w| {
            format!(
                "{{\"first\":{},\"second\":{},\"web_mm\":{}}}",
                w.first,
                w.second,
                json_number(w.web_mm)
            )
        })
        .collect();
    Ok(format!(
        "{{\"openings\":{},\"round\":{},\"rectangle\":{},\"obround\":{},\"paste_ul\":{},\"thickness_mm\":{},\"shrink_percent\":{},\"min_aspect\":{},\"min_area\":{},\"weak\":[{}],\"narrow_webs\":[{}],\"duplicates_removed\":{}}}",
        shown.len(),
        s.round,
        s.rectangle,
        s.obround,
        json_number(s.paste_ul),
        json_number(thickness_mm),
        json_number(shrink_percent),
        opt(min_aspect),
        opt(min_area),
        weak_items.join(","),
        web_items.join(","),
        duplicates_removed
    ))
}
