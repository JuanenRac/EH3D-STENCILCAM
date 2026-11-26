//! The kerf of the laser from the tool crib of the suite. The crib program exports its tools as a CSV table (a header row, commas, double quotes around a text that has a comma, a
//! quote or a line break); a tool of the kind `laser` has the width of its beam in `diameter_mm`, and that is the kerf of the cut. The table is read with limits: a file over
//! [`MAX_CRIB_BYTES`] or with more than [`MAX_CRIB_ROWS`] rows is refused, and every fault names its row.
//!
//! The crib also exports a table of the sheets of stencil material (`export-sheets-csv`, kind `sheet`): [`sheets_from_crib`] reads it with the same limits, by the names of its columns. A sheet gives the
//! thickness, the reduction of every opening on each side, the kerf and the two ratio limits; no figure is a default of this program, and a limit that is 0 gives none.

/// The largest crib file that is read, in bytes.
pub const MAX_CRIB_BYTES: u64 = 1 << 20;
/// The most rows of the table that are read.
pub const MAX_CRIB_ROWS: usize = 10_000;

/// A laser tool of the crib.
#[derive(Debug, Clone, PartialEq)]
pub struct LaserTool {
    pub name: String,
    pub diameter_mm: f64,
}

/// A text that a spreadsheet would run as a formula starts with one of these; the crib writes one more quote mark in front of it.
const FORMULA_START: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

fn unformula(text: &str) -> String {
    match text.strip_prefix('\'') {
        Some(rest) if rest.trim_start_matches('\'').starts_with(FORMULA_START) => rest.to_string(),
        _ => text.to_string(),
    }
}

/// The records of a CSV table: a quoted field may hold commas, line breaks and doubled quotes. An unclosed quote is an error.
fn records(text: &str) -> Result<Vec<Vec<String>>, String> {
    let (mut rows, mut row, mut field) = (Vec::new(), Vec::new(), String::new());
    let (mut quoted, mut was_quoted) = (false, false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => quoted = false,
                c => field.push(c),
            }
            continue;
        }
        match c {
            '"' if field.is_empty() && !was_quoted => {
                quoted = true;
                was_quoted = true;
            }
            ',' => {
                row.push(std::mem::take(&mut field));
                was_quoted = false;
            }
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                was_quoted = false;
                if !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
                if rows.len() > MAX_CRIB_ROWS + 1 {
                    return Err(format!("the crib has more than {MAX_CRIB_ROWS} rows"));
                }
            }
            c => field.push(c),
        }
    }
    if quoted {
        return Err("a quote mark in the crib is never closed".into());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

/// The tools of the kind `laser` of a crib table, in the order of the table. The columns are found by their names (`name`, `kind` and `diameter_mm`), wherever they are.
pub fn laser_tools(csv: &str) -> Result<Vec<LaserTool>, String> {
    let rows = records(csv.trim_start_matches('\u{feff}'))?;
    let (head, body) = rows.split_first().ok_or("the crib is empty")?;
    let column = |name: &str| {
        head.iter()
            .position(|h| h.trim() == name)
            .ok_or_else(|| format!("the crib has no '{name}' column"))
    };
    let (name_at, kind_at, diameter_at) =
        (column("name")?, column("kind")?, column("diameter_mm")?);
    let mut out = Vec::new();
    for (i, row) in body.iter().enumerate() {
        let n = i + 2;
        let cell = |at: usize| {
            row.get(at).ok_or_else(|| {
                format!(
                    "row {n} of the crib has {} cells, fewer than its header",
                    row.len()
                )
            })
        };
        if cell(kind_at)?.trim() != "laser" {
            continue;
        }
        let name = unformula(cell(name_at)?.trim());
        let diameter: f64 = cell(diameter_at)?
            .trim()
            .parse()
            .ok()
            .filter(|d: &f64| d.is_finite())
            .ok_or_else(|| {
                format!("row {n} of the crib: the diameter of the laser '{name}' is not a number")
            })?;
        out.push(LaserTool {
            name,
            diameter_mm: diameter,
        });
    }
    Ok(out)
}

/// The kerf, in mm, of the laser of the crib called `laser` (any case): the width of its beam. The name must be one tool, and its width a number above 0 and up to 1 mm.
pub fn kerf_from_crib(csv: &str, laser: &str) -> Result<f64, String> {
    let wanted = laser.trim().to_lowercase();
    let tools = laser_tools(csv)?;
    let mut found = tools.iter().filter(|t| t.name.to_lowercase() == wanted);
    let tool = found
        .next()
        .ok_or_else(|| format!("the crib has no laser called '{laser}'"))?;
    if found.next().is_some() {
        return Err(format!("the crib has two lasers called '{laser}'"));
    }
    if tool.diameter_mm > 0.0 && tool.diameter_mm <= 1.0 {
        Ok(tool.diameter_mm)
    } else {
        Err(format!(
            "the laser '{}' of the crib has a width of {} mm: the kerf must be above 0 and up to 1 mm",
            tool.name, tool.diameter_mm
        ))
    }
}

/// The text of a crib file, refusing a file over [`MAX_CRIB_BYTES`] before it is read.
fn read_bounded(path: &str) -> Result<String, String> {
    let size = std::fs::metadata(path)
        .map_err(|e| format!("{path}: {e}"))?
        .len();
    if size > MAX_CRIB_BYTES {
        return Err(format!(
            "{path}: {size} bytes is over the limit of {MAX_CRIB_BYTES} bytes for a crib"
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    String::from_utf8(bytes).map_err(|_| format!("{path}: the crib is not UTF-8 text"))
}

/// Reads the crib file, refusing one over [`MAX_CRIB_BYTES`] before it is read, and gives the kerf of the laser called `laser`.
pub fn kerf_from_file(path: &str, laser: &str) -> Result<f64, String> {
    let text = read_bounded(path)?;
    kerf_from_crib(&text, laser).map_err(|e| format!("{path}: {e}"))
}

/// The greatest thickness of a sheet, in mm, as the crib holds it.
pub const MAX_SHEET_THICKNESS_MM: f64 = 50.0;
/// The most an aperture may be made smaller on each side by a sheet, in mm.
pub const MAX_SHEET_REDUCTION_MM: f64 = 0.5;
/// The widest kerf a sheet may hold, in mm.
pub const MAX_SHEET_KERF_MM: f64 = 1.0;
/// The greatest ratio limit a sheet may hold.
pub const MAX_SHEET_RATIO: f64 = 10.0;

/// A sheet of stencil material of the table of sheets that the crib exports (kind `sheet`). A figure that is 0 where 0 is allowed says "none": no reduction, the kerf of a laser tool of
/// the crib, or no limit of that ratio.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetSpec {
    pub name: String,
    pub material: String,
    pub thickness_mm: f64,
    /// How much smaller than the pad each opening is made, on each side, in mm.
    pub reduction_mm: f64,
    /// The width of the cut in this sheet, in mm; 0 takes the kerf of a laser tool of the crib.
    pub kerf_mm: f64,
    /// The least aspect ratio an opening may have in this sheet; 0 gives no limit of its own.
    pub min_aspect_ratio: f64,
    /// The least area ratio an opening may have in this sheet; 0 gives no limit of its own.
    pub min_area_ratio: f64,
}

impl SheetSpec {
    /// Sets the thickness and the reduction of the sheet in a profile; the percent shrink of the material is replaced by the reduction of the sheet. The kerf is the caller's.
    pub fn apply(&self, p: &mut crate::profile::Profile) {
        p.thickness_mm = self.thickness_mm;
        p.shrink_percent = 0.0;
        p.reduction_mm = self.reduction_mm;
    }

    /// The ratio limits with this sheet: a limit the sheet gives replaces the one of `base`, and a limit of 0 leaves `base` as it is. A figure outside the range that this program
    /// takes is an error that names the sheet.
    pub fn limits(&self, base: crate::openings::Limits) -> Result<crate::openings::Limits, String> {
        use crate::openings::{AREA_LIMIT_RANGE, ASPECT_LIMIT_RANGE};
        let pick = |name: &str, v: f64, (lo, hi): (f64, f64), keep: f64| {
            if v == 0.0 {
                Ok(keep)
            } else if (lo..=hi).contains(&v) {
                Ok(v)
            } else {
                Err(format!(
                    "the sheet '{}' asks for {name} {v}: this program takes a limit from {lo} to {hi}",
                    self.name
                ))
            }
        };
        Ok(crate::openings::Limits {
            min_aspect: pick(
                "min_aspect_ratio",
                self.min_aspect_ratio,
                ASPECT_LIMIT_RANGE,
                base.min_aspect,
            )?,
            min_area: pick(
                "min_area_ratio",
                self.min_area_ratio,
                AREA_LIMIT_RANGE,
                base.min_area,
            )?,
        })
    }

    /// One line that says which sheet it is, its material and its figures.
    pub fn summary(&self) -> String {
        format!(
            "sheet '{}' ({}): thickness {} mm, reduction {} mm per side, kerf {}",
            self.name,
            if self.material.is_empty() {
                "no material"
            } else {
                self.material.as_str()
            },
            self.thickness_mm,
            self.reduction_mm,
            if self.kerf_mm > 0.0 {
                format!("{} mm", self.kerf_mm)
            } else {
                String::from("from a laser of the crib")
            }
        )
    }
}

/// The sheets of a table of sheets, in the order of the table. The columns are found by their names (`name`, `kind`, `material`, `thickness_mm`, `reduction_mm`, `kerf_mm`,
/// `min_aspect_ratio` and `min_area_ratio`), wherever they are, and all of them must be there: a table that lacks one would give a sheet with a limit the person never wrote. The kind of
/// every row is `sheet`, the names are one each (any case), and every figure is a number in its range; each fault names its row.
pub fn sheets_from_crib(csv: &str) -> Result<Vec<SheetSpec>, String> {
    let rows = records(csv.trim_start_matches('\u{feff}'))?;
    let (head, body) = rows.split_first().ok_or("the crib is empty")?;
    let column = |name: &str| {
        head.iter()
            .position(|h| h.trim() == name)
            .ok_or_else(|| {
                if name == "thickness_mm" || name == "min_area_ratio" {
                    format!("the crib has no '{name}' column: it is not a table of sheets (the crib exports one with export-sheets-csv)")
                } else {
                    format!("the crib has no '{name}' column")
                }
            })
    };
    let (name_at, kind_at, material_at) = (column("name")?, column("kind")?, column("material")?);
    let thickness_at = column("thickness_mm")?;
    let (reduction_at, kerf_at) = (column("reduction_mm")?, column("kerf_mm")?);
    let (aspect_at, area_at) = (column("min_aspect_ratio")?, column("min_area_ratio")?);
    let mut out: Vec<SheetSpec> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (i, row) in body.iter().enumerate() {
        let n = i + 2;
        let cell = |at: usize| {
            row.get(at).map(|c| c.trim()).ok_or_else(|| {
                format!(
                    "row {n} of the crib has {} cells, fewer than its header",
                    row.len()
                )
            })
        };
        if cell(kind_at)? != "sheet" {
            return Err(format!(
                "row {n} of the crib: the kind is '{}', not 'sheet'",
                cell(kind_at)?
            ));
        }
        let name = unformula(cell(name_at)?);
        if name.is_empty() || name.chars().any(|c| c.is_control()) {
            return Err(format!(
                "row {n} of the crib: the name of a sheet must have text and no control character"
            ));
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(format!(
                "row {n} of the crib: the sheet '{name}' is already in the table"
            ));
        }
        let material = unformula(cell(material_at)?);
        if material.chars().any(|c| c.is_control()) {
            return Err(format!(
                "row {n} of the crib: the material of '{name}' has a control character"
            ));
        }
        let number = |at: usize, what: &str, max: f64, above_zero: bool| -> Result<f64, String> {
            let v: f64 = cell(at)?
                .parse()
                .ok()
                .filter(|v: &f64| v.is_finite())
                .ok_or_else(|| {
                    format!("row {n} of the crib: {what} of the sheet '{name}' is not a number")
                })?;
            if v > max || if above_zero { v <= 0.0 } else { v < 0.0 } {
                return Err(format!(
                    "row {n} of the crib: {what} of the sheet '{name}' is {v}: it must be {} {max}",
                    if above_zero {
                        "more than 0 and at most"
                    } else {
                        "from 0 to"
                    }
                ));
            }
            Ok(v)
        };
        out.push(SheetSpec {
            thickness_mm: number(thickness_at, "thickness_mm", MAX_SHEET_THICKNESS_MM, true)?,
            reduction_mm: number(reduction_at, "reduction_mm", MAX_SHEET_REDUCTION_MM, false)?,
            kerf_mm: number(kerf_at, "kerf_mm", MAX_SHEET_KERF_MM, false)?,
            min_aspect_ratio: number(aspect_at, "min_aspect_ratio", MAX_SHEET_RATIO, false)?,
            min_area_ratio: number(area_at, "min_area_ratio", MAX_SHEET_RATIO, false)?,
            name,
            material,
        });
    }
    Ok(out)
}

/// Every sheet of the table of sheets in a file, refusing one over [`MAX_CRIB_BYTES`] before it is read; every message starts with the path.
pub fn sheets_from_file(path: &str) -> Result<Vec<SheetSpec>, String> {
    let text = read_bounded(path)?;
    sheets_from_crib(&text).map_err(|e| format!("{path}: {e}"))
}

/// The sheet called `name` (any case, spaces around ignored) of a table of sheets.
pub fn sheet_from_crib(csv: &str, name: &str) -> Result<SheetSpec, String> {
    let wanted = name.trim().to_lowercase();
    sheets_from_crib(csv)?
        .into_iter()
        .find(|s| s.name.to_lowercase() == wanted)
        .ok_or_else(|| format!("the crib has no sheet called '{name}'"))
}

/// Reads the table of sheets in a file, refusing one over [`MAX_CRIB_BYTES`] before it is read, and gives the sheet called `name`; every message starts with the path.
pub fn sheet_from_file(path: &str, name: &str) -> Result<SheetSpec, String> {
    let text = read_bounded(path)?;
    sheet_from_crib(&text, name).map_err(|e| format!("{path}: {e}"))
}
