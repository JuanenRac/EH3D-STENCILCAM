//! The settings file of the project, `stencil.cfg`: `key = value`, one per line, `#` starts a comment. Its keys are `lead_in_mm` and `lead_out_mm` (the straight tangent lead-in
//! and lead-out of every cut) and `laser_wait_s` (the wait after the laser is switched on). The command line takes the same settings as options (`--lead-in`, `--lead-out`,
//! `--laser-wait`), which win over the file. A line that makes no sense is an error that says its number.

use crate::output::MAX_LEAD_MM;

/// The default length of the lead-in and of the lead-out, in millimetres.
pub const DEFAULT_LEAD_MM: f64 = 0.3;
/// The most the laser may wait after it is switched on, in seconds.
pub const MAX_LASER_WAIT_S: f64 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The length of the straight lead before each contour, from 0 to 5 mm (0: none).
    pub lead_in_mm: f64,
    /// The length of the straight lead after each contour, from 0 to 5 mm (0: none).
    pub lead_out_mm: f64,
    /// The wait after each `M3`, from 0 to 60 seconds (0: none).
    pub laser_wait_s: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lead_in_mm: DEFAULT_LEAD_MM,
            lead_out_mm: DEFAULT_LEAD_MM,
            laser_wait_s: 0.0,
        }
    }
}

fn number_in(name: &str, text: &str, max: f64, unit: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && (0.0..=max).contains(v))
        .ok_or_else(|| format!("{name} '{text}' must be a number from 0 to {max} {unit}"))
}

/// The value of a lead as the file or the command line gives it: from 0 to 5 mm.
pub fn parse_lead(name: &str, text: &str) -> Result<f64, String> {
    number_in(name, text, MAX_LEAD_MM, "mm")
}

/// The value of the laser wait as the file or the command line gives it: from 0 to 60 seconds.
pub fn parse_wait(name: &str, text: &str) -> Result<f64, String> {
    number_in(name, text, MAX_LASER_WAIT_S, "seconds")
}

/// Reads a settings file. A key that is not known, a line with no `=` and a value that is not valid are errors with the number of the line; a key given twice is an error too.
pub fn parse(text: &str) -> Result<Settings, String> {
    let mut settings = Settings::default();
    let mut seen: Vec<&str> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {n}: '{line}' is not 'key = value'"))?;
        let (key, value) = (key.trim(), value.trim());
        let known = ["lead_in_mm", "lead_out_mm", "laser_wait_s"];
        let Some(&name) = known.iter().find(|k| **k == key) else {
            return Err(format!("line {n}: '{key}' is not a setting"));
        };
        if seen.contains(&name) {
            return Err(format!("line {n}: {name} is given twice"));
        }
        seen.push(name);
        let parsed = match name {
            "laser_wait_s" => parse_wait(name, value),
            _ => parse_lead(name, value),
        }
        .map_err(|e| format!("line {n}: {e}"))?;
        match name {
            "lead_in_mm" => settings.lead_in_mm = parsed,
            "lead_out_mm" => settings.lead_out_mm = parsed,
            _ => settings.laser_wait_s = parsed,
        }
    }
    Ok(settings)
}

/// Every setting with its default, as a file that [`parse`] reads back.
pub fn default_file() -> String {
    let d = Settings::default();
    format!(
        "# EH3D-STENCILCAM settings\n# the straight lead-in before each contour, tangent to its first segment (0 to {MAX_LEAD_MM} mm, 0 for none)\nlead_in_mm = {}\n# the straight lead-out after each contour, tangent to its last segment (0 to {MAX_LEAD_MM} mm, 0 for none)\nlead_out_mm = {}\n# the wait after the laser is switched on (0 to {MAX_LASER_WAIT_S} seconds, 0 for none)\nlaser_wait_s = {}\n",
        d.lead_in_mm, d.lead_out_mm, d.laser_wait_s
    )
}
