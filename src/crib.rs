//! The kerf of the laser from the tool crib of the suite. The crib program exports its tools as a CSV table (a header row, commas, double quotes around a text that has a comma, a
//! quote or a line break); a tool of the kind `laser` has the width of its beam in `diameter_mm`, and that is the kerf of the cut. The table is read with limits: a file over
//! [`MAX_CRIB_BYTES`] or with more than [`MAX_CRIB_ROWS`] rows is refused, and every fault names its row.

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

/// Reads the crib file, refusing one over [`MAX_CRIB_BYTES`] before it is read, and gives the kerf of the laser called `laser`.
pub fn kerf_from_file(path: &str, laser: &str) -> Result<f64, String> {
    let size = std::fs::metadata(path)
        .map_err(|e| format!("{path}: {e}"))?
        .len();
    if size > MAX_CRIB_BYTES {
        return Err(format!(
            "{path}: {size} bytes is over the limit of {MAX_CRIB_BYTES} bytes for a crib"
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let text =
        String::from_utf8(bytes).map_err(|_| format!("{path}: the crib is not UTF-8 text"))?;
    kerf_from_crib(&text, laser).map_err(|e| format!("{path}: {e}"))
}
