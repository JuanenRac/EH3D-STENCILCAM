//! The settings file of the project, `stencil.cfg`: `key = value`, one per line, `#` starts a comment. Its keys are `lead_in_mm` and `lead_out_mm` (the straight tangent lead-in
//! and lead-out of every cut) `laser_wait_s` (the wait after the laser is switched on) and `rapid_mm_min` (the speed of the moves with the laser off, to estimate the time; 0: no estimate). The command line takes the same settings as options (`--lead-in`, `--lead-out`,
//! `--laser-wait`, `--rapid`), which win over the file. `lead_in_round_mm`, `lead_out_round_mm`, `lead_in_rect_mm` and `lead_out_rect_mm` (`--lead-in-round`, `--lead-out-round`, `--lead-in-rect`, `--lead-out-rect`) give the
//! leads of the round and of the rectangular openings; one that is not given is the lead-in or the lead-out of every other opening. `feed_mm_min` (`--feed`, the speed of the cut), `smax`
//! (`--smax`, the top of the laser power scale) and `passes` (`--passes`, how many times every contour is cut, for a thick material) complete the cut; their defaults are the conservative ones
//! (600 mm/min, 1000 and one pass). A line that makes no sense is an error that says its number.

use crate::output::{FormLeads, Lead, MAX_LEAD_MM};

/// The default length of the lead-in and of the lead-out, in millimetres.
pub const DEFAULT_LEAD_MM: f64 = 0.3;
/// The most the laser may wait after it is switched on, in seconds.
pub const MAX_LASER_WAIT_S: f64 = 60.0;
/// The slowest and the fastest speed of the moves with the laser off that may be given, in mm/min (0 gives none).
pub const RAPID_MM_MIN_RANGE: (f64, f64) = (100.0, 100_000.0);
/// The default speed of the cut, in mm/min, and the range that may be given.
pub const DEFAULT_FEED_MM_MIN: f64 = 600.0;
pub const FEED_MM_MIN_RANGE: (f64, f64) = (1.0, 100_000.0);
/// The default top of the laser power scale (`S` of the G-code) and the range that may be given.
pub const DEFAULT_SMAX: f64 = 1000.0;
pub const SMAX_RANGE: (f64, f64) = (1.0, 100_000.0);
/// The most passes over every contour that may be asked for (one is the default).
pub const MAX_PASSES: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The length of the straight lead before each contour, from 0 to 5 mm (0: none).
    pub lead_in_mm: f64,
    /// The length of the straight lead after each contour, from 0 to 5 mm (0: none).
    pub lead_out_mm: f64,
    /// The wait after each `M3`, from 0 to 60 seconds (0: none).
    pub laser_wait_s: f64,
    /// The speed of the moves with the laser off, from 100 to 100000 mm/min, used only to estimate the time (0: no estimate).
    pub rapid_mm_min: f64,
    /// The lead-in of a round opening, from 0 to 5 mm; `None`: the same as `lead_in_mm`.
    pub lead_in_round_mm: Option<f64>,
    /// The lead-out of a round opening, from 0 to 5 mm; `None`: the same as `lead_out_mm`.
    pub lead_out_round_mm: Option<f64>,
    /// The lead-in of a rectangular opening, from 0 to 5 mm; `None`: the same as `lead_in_mm`.
    pub lead_in_rect_mm: Option<f64>,
    /// The lead-out of a rectangular opening, from 0 to 5 mm; `None`: the same as `lead_out_mm`.
    pub lead_out_rect_mm: Option<f64>,
    /// The speed of the cut, from 1 to 100000 mm/min.
    pub feed_mm_min: f64,
    /// The top of the laser power scale (`S`), from 1 to 100000.
    pub smax: f64,
    /// How many times every contour is cut, from 1 to 10 (one by default; more for a thick material).
    pub passes: u32,
}

impl Settings {
    /// The leads by form: a form with no lead of its own takes the general one.
    pub fn leads(&self) -> FormLeads {
        FormLeads {
            general: Lead {
                in_mm: self.lead_in_mm,
                out_mm: self.lead_out_mm,
            },
            round: Lead {
                in_mm: self.lead_in_round_mm.unwrap_or(self.lead_in_mm),
                out_mm: self.lead_out_round_mm.unwrap_or(self.lead_out_mm),
            },
            rect: Lead {
                in_mm: self.lead_in_rect_mm.unwrap_or(self.lead_in_mm),
                out_mm: self.lead_out_rect_mm.unwrap_or(self.lead_out_mm),
            },
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lead_in_mm: DEFAULT_LEAD_MM,
            lead_out_mm: DEFAULT_LEAD_MM,
            laser_wait_s: 0.0,
            rapid_mm_min: 0.0,
            lead_in_round_mm: None,
            lead_out_round_mm: None,
            lead_in_rect_mm: None,
            lead_out_rect_mm: None,
            feed_mm_min: DEFAULT_FEED_MM_MIN,
            smax: DEFAULT_SMAX,
            passes: 1,
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

/// The value of the speed of the rapid moves as the file or the command line gives it: 0 (no estimate) or from 100 to 100000 mm/min.
pub fn parse_rapid(name: &str, text: &str) -> Result<f64, String> {
    let (low, high) = RAPID_MM_MIN_RANGE;
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| *v == 0.0 || (v.is_finite() && (low..=high).contains(v)))
        .ok_or_else(|| {
            format!(
                "{name} '{text}' must be 0 (no estimate) or a number from {low} to {high} mm/min"
            )
        })
}

/// The speed of the cut as the file or the command line gives it: from 1 to 100000 mm/min.
pub fn parse_feed(name: &str, text: &str) -> Result<f64, String> {
    let (low, high) = FEED_MM_MIN_RANGE;
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && (low..=high).contains(v))
        .ok_or_else(|| format!("{name} '{text}' must be a number from {low} to {high} mm/min"))
}

/// The top of the laser power scale as the file or the command line gives it: from 1 to 100000.
pub fn parse_smax(name: &str, text: &str) -> Result<f64, String> {
    let (low, high) = SMAX_RANGE;
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && (low..=high).contains(v))
        .ok_or_else(|| format!("{name} '{text}' must be a number from {low} to {high}"))
}

/// The number of passes as the file or the command line gives it: a whole number from 1 to 10.
pub fn parse_passes(name: &str, text: &str) -> Result<u32, String> {
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|v| (1..=MAX_PASSES).contains(v))
        .ok_or_else(|| format!("{name} '{text}' must be a whole number from 1 to {MAX_PASSES}"))
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
        let known = [
            "lead_in_mm",
            "lead_out_mm",
            "laser_wait_s",
            "rapid_mm_min",
            "lead_in_round_mm",
            "lead_out_round_mm",
            "lead_in_rect_mm",
            "lead_out_rect_mm",
            "feed_mm_min",
            "smax",
            "passes",
        ];
        let Some(&name) = known.iter().find(|k| **k == key) else {
            return Err(format!("line {n}: '{key}' is not a setting"));
        };
        if seen.contains(&name) {
            return Err(format!("line {n}: {name} is given twice"));
        }
        seen.push(name);
        match name {
            "feed_mm_min" => {
                settings.feed_mm_min =
                    parse_feed(name, value).map_err(|e| format!("line {n}: {e}"))?
            }
            "smax" => {
                settings.smax = parse_smax(name, value).map_err(|e| format!("line {n}: {e}"))?
            }
            "passes" => {
                settings.passes = parse_passes(name, value).map_err(|e| format!("line {n}: {e}"))?
            }
            _ => {}
        }
        if matches!(name, "feed_mm_min" | "smax" | "passes") {
            continue;
        }
        let parsed = match name {
            "laser_wait_s" => parse_wait(name, value),
            "rapid_mm_min" => parse_rapid(name, value),
            _ => parse_lead(name, value),
        }
        .map_err(|e| format!("line {n}: {e}"))?;
        match name {
            "lead_in_mm" => settings.lead_in_mm = parsed,
            "lead_out_mm" => settings.lead_out_mm = parsed,
            "laser_wait_s" => settings.laser_wait_s = parsed,
            "rapid_mm_min" => settings.rapid_mm_min = parsed,
            "lead_in_round_mm" => settings.lead_in_round_mm = Some(parsed),
            "lead_out_round_mm" => settings.lead_out_round_mm = Some(parsed),
            "lead_in_rect_mm" => settings.lead_in_rect_mm = Some(parsed),
            _ => settings.lead_out_rect_mm = Some(parsed),
        }
    }
    Ok(settings)
}

/// Every setting with its default, as a file that [`parse`] reads back.
pub fn default_file() -> String {
    let d = Settings::default();
    format!(
        "# EH3D-STENCILCAM settings\n# the straight lead-in before each contour, tangent to its first segment (0 to {MAX_LEAD_MM} mm, 0 for none)\nlead_in_mm = {}\n# the straight lead-out after each contour, tangent to its last segment (0 to {MAX_LEAD_MM} mm, 0 for none)\nlead_out_mm = {}\n# the wait after the laser is switched on (0 to {MAX_LASER_WAIT_S} seconds, 0 for none)\nlaser_wait_s = {}\n# the speed of the moves with the laser off, to estimate the time of the job (100 to 100000 mm/min, 0 for no estimate)\nrapid_mm_min = {}\n# the leads of a round opening and of a rectangular one, when they are not the ones above (0 to {MAX_LEAD_MM} mm); take the # off to use them\n# lead_in_round_mm = {}\n# lead_out_round_mm = {}\n# lead_in_rect_mm = {}\n# lead_out_rect_mm = {}\n# the speed of the cut (1 to 100000 mm/min)\nfeed_mm_min = {}\n# the top of the laser power scale, the S of the G-code (1 to 100000)\nsmax = {}\n# how many times every contour is cut (1 to {MAX_PASSES}; more for a thick material)\npasses = {}\n",
        d.lead_in_mm,
        d.lead_out_mm,
        d.laser_wait_s,
        d.rapid_mm_min,
        d.lead_in_mm,
        d.lead_out_mm,
        d.lead_in_mm,
        d.lead_out_mm,
        d.feed_mm_min,
        d.smax,
        d.passes
    )
}
