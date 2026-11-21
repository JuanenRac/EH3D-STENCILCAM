//! The openings of a stencil written as a drawing, the counterpart of [`crate::dxf_in`]: a round opening is a `CIRCLE` and a rectangular one a closed `LWPOLYLINE` of four corners with
//! the sides along the axes, in millimetres (`$INSUNITS` 4), so that the drawing is read back as the same openings. The text is the same for the same openings: no date, no handle
//! and one layer. An obround opening has no entity that [`crate::dxf_in`] reads, so it is refused unless the caller leaves it out. A drawing is never written over a file that exists
//! unless the caller says so.

use crate::openings::{Form, Opening};
use std::io::Write;
use std::path::Path;

/// The version of the format written: the first one that has the light polyline.
pub const DXF_VERSION: &str = "AC1014";

fn pair(out: &mut String, code: u32, value: &str) {
    out.push_str(&format!("{code:>3}\n{value}\n"));
}

/// The drawing of the openings and how many obround ones were left out. An obround opening is an error unless `leave_out_obround` is set; no opening to draw is an error.
pub fn to_dxf(openings: &[Opening], leave_out_obround: bool) -> Result<(String, usize), String> {
    let obround = openings.iter().filter(|o| o.form == Form::Obround).count();
    if obround > 0 && !leave_out_obround {
        return Err(format!(
            "{obround} obround opening(s) cannot be drawn: a drawing holds circles and rectangles only"
        ));
    }
    let drawn: Vec<&Opening> = openings
        .iter()
        .filter(|o| o.form != Form::Obround)
        .collect();
    if drawn.is_empty() {
        return Err(String::from("there is no opening to draw"));
    }
    let mut s = String::new();
    for (code, value) in [
        (0, "SECTION"),
        (2, "HEADER"),
        (9, "$ACADVER"),
        (1, DXF_VERSION),
        (9, "$INSUNITS"),
        (70, "4"),
        (0, "ENDSEC"),
        (0, "SECTION"),
        (2, "ENTITIES"),
    ] {
        pair(&mut s, code, value);
    }
    for o in drawn {
        if !(o.x.is_finite() && o.y.is_finite() && o.w.is_finite() && o.h.is_finite())
            || o.w <= 0.0
            || o.h <= 0.0
        {
            return Err(format!(
                "the opening at ({}, {}) has no size or place that can be drawn",
                o.x, o.y
            ));
        }
        if o.form == Form::Round {
            pair(&mut s, 0, "CIRCLE");
            pair(&mut s, 8, "0");
            pair(&mut s, 10, &format!("{}", o.x));
            pair(&mut s, 20, &format!("{}", o.y));
            pair(&mut s, 30, "0");
            pair(&mut s, 40, &format!("{}", o.w / 2.0));
        } else {
            let (x0, x1, y0, y1) = (
                o.x - o.w / 2.0,
                o.x + o.w / 2.0,
                o.y - o.h / 2.0,
                o.y + o.h / 2.0,
            );
            pair(&mut s, 0, "LWPOLYLINE");
            pair(&mut s, 100, "AcDbEntity");
            pair(&mut s, 8, "0");
            pair(&mut s, 100, "AcDbPolyline");
            pair(&mut s, 90, "4");
            pair(&mut s, 70, "1");
            for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
                pair(&mut s, 10, &format!("{x}"));
                pair(&mut s, 20, &format!("{y}"));
            }
        }
    }
    pair(&mut s, 0, "ENDSEC");
    pair(&mut s, 0, "EOF");
    Ok((s, obround))
}

/// Writes `text` to a file that does not exist yet. A file that exists is never touched: the error starts with the code `file_exists`. When the write fails the file is removed and the
/// error starts with `file_unwritable`.
pub fn write_new(file: &Path, text: &str) -> Result<(), String> {
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "file_exists: {} already exists; use --overwrite to replace it",
                    file.display()
                )
            } else {
                format!("file_unwritable: {}: {e}", file.display())
            }
        })?;
    if let Err(e) = f.write_all(text.as_bytes()).and_then(|()| f.flush()) {
        drop(f);
        let _ = std::fs::remove_file(file);
        return Err(format!("file_unwritable: {}: {e}", file.display()));
    }
    Ok(())
}
