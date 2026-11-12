//! The openings of a stencil drawn in a DXF file (the text form): a circle (`CIRCLE`) is a round opening and a closed light polyline (`LWPOLYLINE`) of four corners with square corners
//! and sides along the axes is a rectangular one. Anything else that is drawn (lines, arcs, other polylines, text) cannot become an opening and is a note, once each kind. The drawing's own unit
//! (`$INSUNITS`: 1 inches, 2 feet, 4 millimetres, 5 centimetres, 6 metres) is turned into millimetres; a drawing that does not say is taken to be in millimetres.

use crate::openings::{Form, Opening};

/// The extension of a drawing, in lower case.
pub const EXTENSION: &str = "dxf";

/// The most entities of a drawing that are read: a stencil has some thousands of openings at most, and a drawing past this is refused instead of being read for ever.
pub const MAX_ENTITIES: usize = 200_000;

/// The sides of a rectangle may differ from the axes by this much (mm) and still be one.
const AXIS_TOLERANCE_MM: f64 = 0.001;

fn unit_mm(code: i64) -> f64 {
    match code {
        1 => 25.4,
        2 => 304.8,
        5 => 10.0,
        6 => 1000.0,
        _ => 1.0,
    }
}

#[derive(Default)]
struct Entity {
    kind: String,
    closed: bool,
    x: f64,
    y: f64,
    radius: f64,
    vertices: Vec<(f64, f64, f64)>,
}

/// Whether a file name is a drawing (by its extension, in either case).
pub fn is_drawing(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

/// The openings of a drawing and what could not become one.
pub fn from_dxf(text: &str) -> Result<(Vec<Opening>, Vec<String>), String> {
    let mut lines = text.lines();
    let mut pairs: Vec<(i64, String)> = Vec::new();
    while let Some(code) = lines.next() {
        let Some(value) = lines.next() else { break };
        let code = code
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("'{}' is not a group code of a DXF file", code.trim()))?;
        pairs.push((code, value.trim().to_string()));
    }
    if pairs.is_empty() {
        return Err(String::from("the drawing is empty"));
    }
    let number = |s: &str| s.parse::<f64>().ok().filter(|v| v.is_finite());
    let mut unit = 1.0;
    let mut entities: Vec<Entity> = Vec::new();
    let mut in_entities = false;
    let mut current: Option<Entity> = None;
    let mut pending_unit = false;
    let mut vertex: Option<(f64, f64, f64)> = None;
    for (code, value) in &pairs {
        if *code == 0 {
            if let Some(v) = vertex.take() {
                if let Some(e) = current.as_mut() {
                    e.vertices.push(v);
                }
            }
            if let Some(e) = current.take() {
                entities.push(e);
                if entities.len() > MAX_ENTITIES {
                    return Err(format!("the drawing has more than {MAX_ENTITIES} entities"));
                }
            }
            match value.as_str() {
                "SECTION" | "ENDSEC" | "EOF" => {
                    if value == "ENDSEC" {
                        in_entities = false;
                    }
                }
                kind if in_entities => {
                    current = Some(Entity {
                        kind: kind.to_string(),
                        ..Entity::default()
                    })
                }
                _ => {}
            }
            continue;
        }
        if *code == 2 && value == "ENTITIES" {
            in_entities = true;
            continue;
        }
        if *code == 9 {
            pending_unit = value == "$INSUNITS";
            continue;
        }
        if pending_unit && *code == 70 {
            unit = unit_mm(value.parse::<i64>().unwrap_or(4));
            pending_unit = false;
            continue;
        }
        let Some(e) = current.as_mut() else { continue };
        match (e.kind.as_str(), *code) {
            ("CIRCLE", 40) => e.radius = number(value).unwrap_or(0.0),
            ("CIRCLE", 10) => e.x = number(value).unwrap_or(0.0),
            ("CIRCLE", 20) => e.y = number(value).unwrap_or(0.0),
            ("LWPOLYLINE", 70) => {
                e.closed = value.parse::<i64>().map(|f| f & 1 == 1).unwrap_or(false)
            }
            ("LWPOLYLINE", 10) => {
                if let Some(v) = vertex.take() {
                    e.vertices.push(v);
                }
                vertex = Some((number(value).unwrap_or(f64::NAN), 0.0, 0.0));
            }
            ("LWPOLYLINE", 20) => {
                if let Some(v) = vertex.as_mut() {
                    v.1 = number(value).unwrap_or(f64::NAN);
                }
            }
            ("LWPOLYLINE", 42) => {
                if let Some(v) = vertex.as_mut() {
                    v.2 = number(value).unwrap_or(0.0);
                }
            }
            _ => {}
        }
    }
    if let Some(v) = vertex.take() {
        if let Some(e) = current.as_mut() {
            e.vertices.push(v);
        }
    }
    if let Some(e) = current.take() {
        entities.push(e);
    }
    let (mut openings, mut notes) = (Vec::new(), Vec::new());
    let mut skipped: Vec<String> = Vec::new();
    for e in &entities {
        match e.kind.as_str() {
            "CIRCLE" if e.radius > 0.0 => openings.push(Opening {
                x: e.x * unit,
                y: e.y * unit,
                w: e.radius * 2.0 * unit,
                h: e.radius * 2.0 * unit,
                form: Form::Round,
            }),
            "CIRCLE" => note_once(&mut skipped, &mut notes, "CIRCLE", "a circle with no radius is not an opening"),
            "LWPOLYLINE" => match rectangle(e, unit) {
                Some(o) => openings.push(o),
                None => note_once(
                    &mut skipped,
                    &mut notes,
                    "LWPOLYLINE",
                    "a polyline that is not a closed rectangle with its sides along the axes is not an opening",
                ),
            },
            "VERTEX" | "SEQEND" | "POINT" => {}
            other => note_once(
                &mut skipped,
                &mut notes,
                other,
                "this kind of entity cannot become an opening (only circles and rectangles can)",
            ),
        }
    }
    if openings.is_empty() && notes.is_empty() {
        notes.push(String::from(
            "the drawing has no circle or rectangle: there is nothing to cut",
        ));
    }
    Ok((openings, notes))
}

fn note_once(seen: &mut Vec<String>, notes: &mut Vec<String>, kind: &str, why: &str) {
    if !seen.iter().any(|k| k == kind) {
        seen.push(kind.to_string());
        notes.push(format!("dxf {kind}: {why}"));
    }
}

fn rectangle(e: &Entity, unit: f64) -> Option<Opening> {
    if !e.closed
        || e.vertices.len() != 4
        || e.vertices
            .iter()
            .any(|v| !v.0.is_finite() || !v.1.is_finite() || v.2 != 0.0)
    {
        return None;
    }
    let v: Vec<(f64, f64)> = e
        .vertices
        .iter()
        .map(|p| (p.0 * unit, p.1 * unit))
        .collect();
    let (x0, x1) = (
        v.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
        v.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
    );
    let (y0, y1) = (
        v.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
        v.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
    );
    // every corner is a corner of the box, and the box has a width and a height
    let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    let on_a_corner = |p: &(f64, f64)| {
        corners.iter().any(|c| {
            (c.0 - p.0).abs() <= AXIS_TOLERANCE_MM && (c.1 - p.1).abs() <= AXIS_TOLERANCE_MM
        })
    };
    if x1 - x0 <= AXIS_TOLERANCE_MM || y1 - y0 <= AXIS_TOLERANCE_MM || !v.iter().all(on_a_corner) {
        return None;
    }
    Some(Opening {
        x: (x0 + x1) / 2.0,
        y: (y0 + y1) / 2.0,
        w: x1 - x0,
        h: y1 - y0,
        form: Form::Rectangle,
    })
}
