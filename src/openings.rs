//! The openings of the stencil: one for every pad that the paste layer flashes.

use crate::profile::Profile;
use eh3d_parser_gerber::{Aperture, GerberFile, Operation, Units};

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

/// The openings of a paste layer and what could not be turned into one (with the reason, once each).
pub fn from_gerber(file: &GerberFile) -> (Vec<Opening>, Vec<String>) {
    let k = file.units.unwrap_or(Units::Millimetres).to_mm(1.0);
    let (mut out, mut notes) = (Vec::new(), Vec::new());
    let note = |text: &str, notes: &mut Vec<String>| {
        if !notes.iter().any(|n| n == text) {
            notes.push(text.to_string());
        }
    };
    let mut current: Option<&Aperture> = None;
    for c in &file.commands {
        match &c.op {
            Operation::SelectAperture(n) => current = file.apertures.get(n),
            Operation::Flash { at } => {
                let (x, y) = (at.x * k, at.y * k);
                match current {
                    Some(Aperture::Circle { diameter, .. }) => out.push(Opening {
                        x,
                        y,
                        w: diameter * k,
                        h: diameter * k,
                        form: Form::Round,
                    }),
                    Some(Aperture::Rectangle { x: w, y: h, .. }) => out.push(Opening {
                        x,
                        y,
                        w: w * k,
                        h: h * k,
                        form: Form::Rectangle,
                    }),
                    Some(Aperture::Obround { x: w, y: h, .. }) => out.push(Opening {
                        x,
                        y,
                        w: w * k,
                        h: h * k,
                        form: Form::Obround,
                    }),
                    Some(_) => note(
                        "a polygon or macro pad is not turned into an opening",
                        &mut notes,
                    ),
                    None => note("a flash with no aperture selected is skipped", &mut notes),
                }
            }
            Operation::Line { .. } | Operation::Arc { .. } | Operation::RegionStart => note(
                "tracks and regions of a paste layer are not turned into openings",
                &mut notes,
            ),
            _ => {}
        }
    }
    out.retain(|o| o.w > 0.0 && o.h > 0.0);
    (out, notes)
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
    let mut out = Vec::new();
    for (i, o) in openings.iter().enumerate() {
        let r = ratios(o, thickness_mm)?;
        if r.aspect < MIN_ASPECT {
            out.push((i, format!("opening {} ({:.3} x {:.3} mm at ({:.3}, {:.3})): aspect ratio {:.2} is below {MIN_ASPECT}; the paste will stay in it", i + 1, o.w, o.h, o.x, o.y, r.aspect)));
        } else if r.area < MIN_AREA {
            out.push((i, format!("opening {} ({:.3} x {:.3} mm at ({:.3}, {:.3})): area ratio {:.2} is below {MIN_AREA}; the paste will stay in it", i + 1, o.w, o.h, o.x, o.y, r.area)));
        }
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
