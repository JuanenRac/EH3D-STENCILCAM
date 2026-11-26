// EH3D-STENCILCAM - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_parser_gcode::emit::{header, sha256_hex, Header};
use eh3d_project::manufacturing as suite;
use eh3d_serverclient::Settings as LinkSettings;
use eh3d_stencilcam::config;
use eh3d_stencilcam::crib::SheetSpec;
use eh3d_stencilcam::dxf_in::{from_dxf, is_drawing};
use eh3d_stencilcam::dxf_out;
use eh3d_stencilcam::eh3dfile;
use eh3d_stencilcam::layer::{hidden_codes, read_layer, read_layer_file};
use eh3d_stencilcam::openings;
use eh3d_stencilcam::openings::{
    check_json, deduplicated, from_gerber_rules, mirrored, narrow_webs, ratios, shrunk, summary,
    weak_with,
};
use eh3d_stencilcam::output::{
    duration_text, estimate_time_with_forms, forms, gcode_with_form_leads, hpgl, move_to_origin,
    outside_bed, paths, write_atomic, FormLeads, Lead,
};
use eh3d_stencilcam::profile::{Material, Profile};
use eh3d_stencilcam::server;
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-stencilcam: solder paste stencils from the paste layer of a board

  eh3d-stencilcam convert LAYER.gtp|DRAWING.dxf OUT.nc|OUT.plt [--material mylar|kapton|steel] [--thickness MM] [--shrink PERCENT] [--radius MM]
                  [--kerf MM | --crib FILE.csv --crib-laser NAME] [--sheet-file SHEETS.csv --sheet NAME] [--feed 600] [--smax 1000] [--passes 1] [--laser-wait SECONDS] [--lead-in MM] [--lead-out MM]
                  [--lead-in-round MM] [--lead-out-round MM] [--lead-in-rect MM] [--lead-out-rect MM] [--rapid MM_MIN] [--origin X,Y] [--bed WIDTH,HEIGHT] [--config FILE] [--mirror] [--force] [--create | --overwrite]
  eh3d-stencilcam project info FILE.eh3d            the paste layers of the job of a project, the stencils kept in it and whether they are out of date (then it ends with 2)
  eh3d-stencilcam project convert FILE.eh3d [--layer NAME] [--plt] [options of convert]   a stencil for each paste layer of the job (or the one named), kept in the branch stencilcam/ of the project
  eh3d-stencilcam config                            every setting with its default, as a file (stencil.cfg)
  eh3d-stencilcam server [status] [--config FILE]   the link to the EH3D server (server_address, server_token, server_machine, server_timeout_s, server_retries, server_mdns and server_max_answer_mib of stencil.cfg) and what the server answers
  eh3d-stencilcam check LAYER.gtp|DRAWING.dxf [--material ...] [--thickness MM] [--shrink PERCENT] [--min-web MM] [--sheet-file SHEETS.csv --sheet NAME] [--min-aspect N] [--min-area N] [--region-fit PERCENT] [--json]
  eh3d-stencilcam [gui] [LAYER.gtp]  the window (a build with --features gui): with no arguments, or with a single layer, it opens; the commands above never open it
  eh3d-stencilcam gui [FILE...] --shot OUT.png [--size WxH] [--theme forge|dark|light] [--wait N] [--lang CODE] [--replace]   a picture of the window (a build with --features gui)

Both read the layer with a size limit (--max-bytes N, 268435456 by default; the refusal is written in the language of the system) and show what the reader says about the file, each notice with its code; --hide CODE[,CODE] leaves
out the notices of those codes (for example units-missing). --arc-tolerance MM (0.005 by default) is how closely the arcs of the layer are followed.

--to-server (convert, and check --json) keeps the file that convert writes, or the JSON of check, on the EH3D server as an artifact, with the values it was cut with; it needs --config FILE with server_address and server_token and is refused before any work when they are missing.
--laser-wait SECONDS writes a G4 wait after each M3 so that the laser reaches its power before the head moves (0 by default: no wait).
--lead-in MM and --lead-out MM (0.3 by default, 0 to 5) add a straight lead-in before each contour and a lead-out after it, tangent to the contour and outside the opening, with the laser off until the start of the lead-in; 0 writes none. G-code only.
--lead-in-round MM, --lead-out-round MM, --lead-in-rect MM and --lead-out-rect MM give the leads of the round openings and of the rectangular ones (0 to 5); one that is not given is the --lead-in or --lead-out value. The obround openings use --lead-in and --lead-out.
--crib FILE.csv --crib-laser NAME takes the kerf from the beam width of a laser tool in the CSV that the tool crib of the suite exports (a file over 1 MiB is refused); it replaces --kerf.
--sheet-file SHEETS.csv --sheet NAME (or sheet_file and sheet_name of the settings file) takes a sheet of the table of sheets that the tool crib exports (export-sheets-csv; a file over 1 MiB is refused): its thickness, its reduction of every opening on each side (replacing the percent of --shrink, which is then refused), its kerf (a kerf of 0 takes the beam width of the laser of --crib and --crib-laser, or --kerf) and the ratio limits it gives (a limit of 0 gives none, and --min-aspect and --min-area win over the sheet, which wins over the settings file). --thickness is refused with a sheet; --kerf and --crib are refused when the sheet has a kerf of its own.
--rapid MM_MIN (0 by default: no estimate; otherwise 100 to 100000) is the speed of the moves with the laser off: convert then prints the estimated time of the G-code job.
--origin X,Y moves the stencil so that the lower-left corner of its openings is at X,Y (the coordinates of the board are used as they are when it is not given). --bed WIDTH,HEIGHT refuses to write when a point of a contour falls outside the bed 0..WIDTH by 0..HEIGHT (the leads are not counted).
--config FILE reads stencil.cfg (lead_in_mm, lead_out_mm, lead_in_round_mm, lead_out_round_mm, lead_in_rect_mm, lead_out_rect_mm, laser_wait_s, rapid_mm_min, feed_mm_min, smax, passes, min_web_mm, min_aspect, min_area, region_fit_percent); the options win over the file.
--min-aspect N (0.5 to 10, 1.5 by default) and --min-area N (0.1 to 2, 0.66 by default) are the limits under which an opening does not release the paste; each reason in check and in convert names the ratio, its limit and the thickest sheet for which the opening would do. A track along an axis and a region that is a rectangle become openings; --region-fit PERCENT (0 to 20, 0 by default) also opens a region that fills at least 100 minus PERCENT of its enclosing rectangle (a pad with rounded corners) as that rectangle, with a note.
--min-web MM (0.1 by default, 0 to 5; 0 gives no warning) warns, in check and in convert, of two openings whose web (the sheet that stays between them) is narrower, or that touch or overlap; it never stops convert. Two openings of the same form, size and place are one hole: the second is left out, with a note on the standard error. check prints how many openings there are of each form and the paste they hold; check --json prints that and the rest of the result as one JSON object.
--feed MM_MIN (1 to 30000, 600 by default) is the speed of the cut, --smax N (1 to 100000, 1000 by default) the top of the laser power scale and --passes N (1 to 10, 1 by default) how many times every contour is cut, all the contours once before the next pass.
A file ending in .dxf is a drawing: its circles are round openings and its closed rectangles with the sides along the axes are rectangular ones (the unit of the drawing is read); any other entity is a note and, as for a layer, convert refuses to write unless --force is given.

OUT ending in .plt is HP-GL for a plotter, OUT ending in .dxf is the drawing of the openings as the stencil has them (circles and rectangles in mm, the counterpart of the drawing that convert reads; an obround opening is refused unless --force leaves it out; --create, the default, refuses a file that exists and --overwrite replaces it; --origin and --bed do not apply); anything else is G-code for a laser. convert refuses to write when an opening cannot release the paste (aspect ratio under 1.5 or
area ratio under 0.66), or when the layer has pads, tracks or regions that cannot become openings, unless --force is given. --mirror is for the stencil of the bottom side.

Exit codes: 0 done; 1 a mistake in what was asked or a file that cannot be read or written; 2 a check that did not pass (an opening that will not release the paste in check or convert, or a layer that cannot all become openings).";

/// The mark in front of an error that is a check that did not pass (the exit code is 2) and not a mistake in what was asked (the exit code is 1).
const NOT_PASSED: char = '\u{1}';

fn not_passed(text: String) -> String {
    format!("{NOT_PASSED}{text}")
}

/// The link to the EH3D server for `--to-server`: the keys of the settings file, which must name a server and hold its token.
fn link_settings(config: Option<&str>) -> Result<LinkSettings, String> {
    let path =
        config.ok_or("--to-server needs --config FILE with server_address and server_token")?;
    let settings = server::settings_from_file(path)?;
    server::require(&settings)?;
    Ok(settings)
}

/// `server [status] [--config FILE]`: the link, with the token as `***`, and what the server answers.
fn server_command(mut args: Vec<String>) -> Result<(), String> {
    let config = flag(&mut args, "--config")?;
    let word = args.first().cloned().unwrap_or_else(|| "status".into());
    finish(&args, 1)?;
    if word != "status" {
        return Err(format!("server {word}: unknown (use: server status)"));
    }
    let settings = match config {
        Some(path) => server::settings_from_file(&path)?,
        None => LinkSettings::default(),
    };
    print!("{}", server::status_report(settings)?);
    Ok(())
}

/// Keeps a file on the server and says so; the file was already written, so a refusal is told as such.
fn keep(
    link: &LinkSettings,
    name: &str,
    format: &str,
    data: &[u8],
    params: &[(&str, &str)],
) -> Result<(), String> {
    let line = server::keep(link.clone(), name, format, data, params)
        .map_err(|e| format!("{name}: not kept on the server: {e}"))?;
    println!("{line}");
    Ok(())
}

/// An option that is two numbers, `X,Y`.
fn pair_arg(name: &str, text: &str) -> Result<(f64, f64), String> {
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("{name} needs two numbers, X,Y"));
    }
    Ok((num(parts[0], name)?, num(parts[1], name)?))
}

/// A lead of one form: the option when it is given, otherwise what the file says (`None`: the lead of every other opening).
fn lead_option(
    args: &mut Vec<String>,
    name: &str,
    from_file: Option<f64>,
) -> Result<Option<f64>, String> {
    match flag(args, name)? {
        Some(v) => config::parse_lead(name, &v).map(Some),
        None => Ok(from_file),
    }
}

/// The profile of the material and of the sheet, and the sheet when one was chosen (`--sheet-file` and `--sheet`, or `sheet_file` and `sheet_name` of the settings file).
fn profile(
    args: &mut Vec<String>,
    cfg_sheet: (Option<String>, Option<String>),
) -> Result<(Profile, Option<SheetSpec>), String> {
    let mut p = Profile::of(match flag(args, "--material")? {
        Some(m) => Material::parse(&m)?,
        None => Material::Mylar,
    });
    let thickness = flag(args, "--thickness")?;
    let shrink = flag(args, "--shrink")?;
    let sheet_file = flag(args, "--sheet-file")?.or(cfg_sheet.0);
    let sheet_name = flag(args, "--sheet")?.or(cfg_sheet.1);
    let sheet = match (&sheet_file, &sheet_name) {
        (Some(file), Some(name)) => Some(eh3d_stencilcam::crib::sheet_from_file(file, name)?),
        (None, None) => None,
        _ => {
            return Err(
                "the sheet file and the sheet name go together (--sheet-file and --sheet, or sheet_file and sheet_name)"
                    .into(),
            )
        }
    };
    if let Some(s) = &sheet {
        if thickness.is_some() || shrink.is_some() {
            return Err(format!(
                "the sheet '{}' gives the thickness and the reduction: do not give {} too",
                s.name,
                if thickness.is_some() {
                    "--thickness"
                } else {
                    "--shrink"
                }
            ));
        }
        s.apply(&mut p);
    }
    if let Some(v) = thickness {
        p.thickness_mm = num(&v, "--thickness")?;
    }
    if let Some(v) = shrink {
        p.shrink_percent = num(&v, "--shrink")?;
    }
    if let Some(v) = flag(args, "--radius")? {
        p.corner_radius_mm = num(&v, "--radius")?;
    }
    let crib = flag(args, "--crib")?;
    let laser = flag(args, "--crib-laser")?;
    let kerf = flag(args, "--kerf")?;
    let crib_kerf = match (&crib, &laser) {
        (Some(file), Some(name)) => {
            if kerf.is_some() {
                return Err("--kerf and --crib give the kerf twice: use one".into());
            }
            Some(eh3d_stencilcam::crib::kerf_from_file(file, name)?)
        }
        (None, None) => None,
        _ => return Err("--crib FILE and --crib-laser NAME go together".into()),
    };
    let kerf = kerf.map(|v| num(&v, "--kerf")).transpose()?;
    match &sheet {
        Some(s) if s.kerf_mm > 0.0 => {
            if crib_kerf.is_some() || kerf.is_some() {
                return Err(format!(
                    "the sheet '{}' has a kerf of its own ({} mm): do not give --kerf or --crib too",
                    s.name, s.kerf_mm
                ));
            }
            p.kerf_mm = s.kerf_mm;
        }
        Some(s) => {
            p.kerf_mm = crib_kerf.or(kerf).ok_or_else(|| {
                format!(
                    "the sheet '{}' takes its kerf from a laser of the crib: give --crib FILE and --crib-laser NAME (or --kerf MM)",
                    s.name
                )
            })?;
        }
        None => {
            if let Some(k) = crib_kerf.or(kerf) {
                p.kerf_mm = k;
            }
        }
    }
    p.validate().map_err(|e| match &sheet {
        Some(s) => format!("sheet '{}': {e}", s.name),
        None => e,
    })?;
    Ok((p, sheet))
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    if cmd == "gui" {
        // a build with the window never gets here (main opens it): this one is only a command line
        return Err("this build has no window: build it with --features gui".into());
    }
    if cmd == "config" {
        finish(&args, 0)?;
        print!("{}", config::default_file());
        return Ok(());
    }
    if cmd == "project" {
        return project(args);
    }
    if cmd == "server" {
        return server_command(args);
    }
    let mut settings = config::Settings::default();
    let mut cfg_sheet = (None, None);
    let config_path = flag(&mut args, "--config")?;
    if let Some(path) = &config_path {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        settings = config::parse(&text).map_err(|e| format!("{path}: {e}"))?;
        cfg_sheet = config::sheet_choice(&text);
    }
    let lead_in = match flag(&mut args, "--lead-in")? {
        Some(v) => config::parse_lead("--lead-in", &v)?,
        None => settings.lead_in_mm,
    };
    let lead_out = match flag(&mut args, "--lead-out")? {
        Some(v) => config::parse_lead("--lead-out", &v)?,
        None => settings.lead_out_mm,
    };
    let round_in = lead_option(&mut args, "--lead-in-round", settings.lead_in_round_mm)?;
    let round_out = lead_option(&mut args, "--lead-out-round", settings.lead_out_round_mm)?;
    let rect_in = lead_option(&mut args, "--lead-in-rect", settings.lead_in_rect_mm)?;
    let rect_out = lead_option(&mut args, "--lead-out-rect", settings.lead_out_rect_mm)?;
    let leads = FormLeads {
        general: Lead {
            in_mm: lead_in,
            out_mm: lead_out,
        },
        round: Lead {
            in_mm: round_in.unwrap_or(lead_in),
            out_mm: round_out.unwrap_or(lead_out),
        },
        rect: Lead {
            in_mm: rect_in.unwrap_or(lead_in),
            out_mm: rect_out.unwrap_or(lead_out),
        },
    };
    let (p, sheet) = profile(&mut args, cfg_sheet)?;
    let origin = flag(&mut args, "--origin")?
        .map(|v| pair_arg("--origin", &v))
        .transpose()?;
    let bed = flag(&mut args, "--bed")?
        .map(|v| pair_arg("--bed", &v))
        .transpose()?;
    if bed.is_some_and(|b| b.0 <= 0.0 || b.1 <= 0.0) {
        return Err("--bed needs a width and a height above 0".into());
    }
    let min_web = match flag(&mut args, "--min-web")? {
        Some(v) => config::parse_min_web("--min-web", &v)?,
        None => settings.min_web_mm,
    };
    // the command line wins over the sheet, and the sheet over the settings file
    let base = openings::Limits {
        min_aspect: settings.min_aspect,
        min_area: settings.min_area,
    };
    let base = match &sheet {
        Some(s) => s.limits(base)?,
        None => base,
    };
    let limits = openings::Limits {
        min_aspect: match flag(&mut args, "--min-aspect")? {
            Some(v) => config::parse_min_aspect("--min-aspect", &v)?,
            None => base.min_aspect,
        },
        min_area: match flag(&mut args, "--min-area")? {
            Some(v) => config::parse_min_area("--min-area", &v)?,
            None => base.min_area,
        },
    };
    let rules = openings::ShapeRules {
        region_fit_percent: match flag(&mut args, "--region-fit")? {
            Some(v) => config::parse_region_fit("--region-fit", &v)?,
            None => settings.region_fit_percent,
        },
    };
    let json = switch(&mut args, "--json");
    let to_server = switch(&mut args, "--to-server");
    if to_server && !(cmd == "convert" || (cmd == "check" && json)) {
        return Err("--to-server is an option of convert and of check --json".into());
    }
    // the link is checked before any work is done
    let link = if to_server {
        Some(link_settings(config_path.as_deref())?)
    } else {
        None
    };
    let mirror = switch(&mut args, "--mirror");
    let force = switch(&mut args, "--force");
    let create = switch(&mut args, "--create");
    let overwrite = switch(&mut args, "--overwrite");
    if create && overwrite {
        return Err("--create and --overwrite say opposite things: use one".into());
    }
    if (create || overwrite) && cmd != "convert" {
        return Err("--create and --overwrite are options of convert".into());
    }
    let feed = match flag(&mut args, "--feed")? {
        Some(v) => config::parse_feed("--feed", &v)?,
        None => settings.feed_mm_min,
    };
    let smax = match flag(&mut args, "--smax")? {
        Some(v) => config::parse_smax("--smax", &v)?,
        None => settings.smax,
    };
    let passes = match flag(&mut args, "--passes")? {
        Some(v) => config::parse_passes("--passes", &v)?,
        None => settings.passes,
    };
    let laser_wait = flag(&mut args, "--laser-wait")?
        .map(|v| num(&v, "--laser-wait"))
        .transpose()?
        .unwrap_or(settings.laser_wait_s);
    if !(0.0..=60.0).contains(&laser_wait) {
        return Err(format!(
            "--laser-wait {laser_wait} must be from 0 to 60 seconds"
        ));
    }
    let rapid = match flag(&mut args, "--rapid")? {
        Some(v) => config::parse_rapid("--rapid", &v)?,
        None => settings.rapid_mm_min,
    };
    let max_bytes = flag(&mut args, "--max-bytes")?
        .map(|v| num(&v, "--max-bytes").map(|n| n as usize))
        .transpose()?
        .unwrap_or(eh3d_parser_gerber::InputLimits::default().max_bytes);
    let arc_tolerance = flag(&mut args, "--arc-tolerance")?
        .map(|v| num(&v, "--arc-tolerance"))
        .transpose()?
        .unwrap_or(eh3d_gerberviewer::scene::DEFAULT_ARC_TOLERANCE_MM);
    if !(0.0001..=1.0).contains(&arc_tolerance) {
        return Err(format!(
            "--arc-tolerance {arc_tolerance} must be from 0.0001 to 1 mm"
        ));
    }
    let hide = flag(&mut args, "--hide")?
        .map(|v| hidden_codes(&v))
        .transpose()?
        .unwrap_or_default();
    let layer = args
        .first()
        .ok_or_else(|| format!("{cmd} needs a paste layer"))?
        .clone();
    finish(&args, if cmd == "convert" { 2 } else { 1 })?;
    // the output may not be the layer: the Gerber file would be overwritten with the G-code
    if cmd == "convert" {
        if let Some(out) = args.get(1) {
            let same = match (
                std::path::Path::new(out).canonicalize(),
                std::path::Path::new(&layer).canonicalize(),
            ) {
                (Ok(a), Ok(b)) => a == b,
                _ => *out == layer,
            };
            if same {
                return Err(format!(
                    "{out}: the output file is also the layer; write the stencil to a new file"
                ));
            }
        }
    }
    let bytes = read_layer_file(&layer, max_bytes, eh3d_stencilcam::layer::language())
        .map_err(|e| format!("{layer}: {e}"))?;
    let drawing = is_drawing(&layer);
    let (mut openings, mut notes, reader_notes, reader_errors) = if drawing {
        let text = String::from_utf8_lossy(&bytes);
        let (o, n) = from_dxf(&text).map_err(|e| format!("{layer}: {e}"))?;
        (o, n, Vec::new(), 0)
    } else {
        let reading = read_layer(&bytes, max_bytes, &hide).map_err(|e| format!("{layer}: {e}"))?;
        let shaped = from_gerber_rules(&reading.file, arc_tolerance, &rules);
        let mut shown = reading.shown;
        shown.extend(shaped.approximated);
        (shaped.openings, shaped.notes, shown, reading.errors)
    };
    // the same hole twice is cut once: the second burns the edge of the first
    let (unique, duplicates_removed) = deduplicated(&openings, 0.001);
    if duplicates_removed > 0 {
        openings = unique;
        eprintln!("note: {duplicates_removed} opening(s) repeat another one (same form, size and place) and are left out");
    }
    if mirror {
        openings = mirrored(&openings);
    }
    for n in &notes {
        eprintln!("note: {n}");
    }
    for n in &reader_notes {
        eprintln!("note: {n}");
    }
    // what the reader lost is lost from the stencil too
    for _ in 0..reader_errors {
        notes.push(String::from("reader error"));
    }
    // a stencil that leaves out pads is not the stencil of the board: it is written only when it is asked for
    if cmd == "convert" && !notes.is_empty() && !force {
        return Err(not_passed(format!(
            "{} part(s) of the layer cannot become openings (see the notes above): the stencil would be incomplete; use --force to write it anyway",
            notes.len()
        )));
    }
    let shown: Vec<_> = openings
        .iter()
        .map(|o| shrunk(o, &p))
        .collect::<Result<_, _>>()?;
    let problems = weak_with(&shown, p.thickness_mm, &limits)?;
    let webs = narrow_webs(&shown, min_web);
    if json && cmd != "check" {
        return Err("--json is an option of check".into());
    }
    if let Some(s) = &sheet {
        // the JSON of check is the whole of the standard output
        if json {
            eprintln!("note: {}", s.summary());
        } else {
            println!("{}", s.summary());
        }
    }
    for w in &webs {
        if w.web_mm <= 0.0 {
            eprintln!(
                "warning: openings {} and {} touch or overlap",
                w.first, w.second
            );
        } else {
            eprintln!(
                "warning: openings {} and {} are {:.3} mm apart (less than {min_web} mm): the sheet between them may break",
                w.first, w.second, w.web_mm
            );
        }
    }
    match cmd.as_str() {
        "check" if json => {
            let report = check_json(
                &shown,
                p.thickness_mm,
                p.shrink_percent,
                &problems,
                &webs,
                duplicates_removed,
            )?;
            println!("{report}");
            if let Some(link) = &link {
                let stem = std::path::Path::new(&layer)
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("layer");
                let (thickness, shrink) =
                    (p.thickness_mm.to_string(), p.shrink_percent.to_string());
                keep(
                    link,
                    &format!("{stem}.check.json"),
                    "application/json",
                    report.as_bytes(),
                    &[
                        ("command", "check"),
                        ("thickness_mm", thickness.as_str()),
                        ("shrink_percent", shrink.as_str()),
                    ],
                )?;
            }
            if problems.is_empty() {
                Ok(())
            } else {
                Err(not_passed(format!(
                    "{} openings will not release the paste",
                    problems.len()
                )))
            }
        }
        "check" => {
            let all = shown
                .iter()
                .map(|o| ratios(o, p.thickness_mm))
                .collect::<Result<Vec<_>, _>>()?;
            println!(
                "{} openings; thickness {} mm, shrink {} %",
                openings.len(),
                p.thickness_mm,
                p.shrink_percent
            );
            if let Some(a) = all.iter().map(|r| r.aspect).reduce(f64::min) {
                println!("smallest aspect ratio {a:.2} (at least {}), smallest area ratio {:.2} (at least {})", limits.min_aspect, all.iter().map(|r| r.area).fold(f64::INFINITY, f64::min), limits.min_area);
            }
            let s = summary(&shown, p.thickness_mm);
            println!(
                "{} round, {} rectangular, {} obround; the openings hold {:.3} uL of paste",
                s.round, s.rectangle, s.obround, s.paste_ul
            );
            for (_, m) in &problems {
                println!("{m}");
            }
            if problems.is_empty() {
                Ok(())
            } else {
                Err(not_passed(format!(
                    "{} openings will not release the paste",
                    problems.len()
                )))
            }
        }
        "convert" => {
            let out = args.get(1).ok_or("convert needs an output file")?;
            if !problems.is_empty() && !force {
                return Err(not_passed(format!("{} openings will not release the paste (run check to see them, or --force):\n{}", problems.len(), problems.iter().map(|(_, m)| m.as_str()).collect::<Vec<_>>().join("\n"))));
            }
            if is_drawing(out) {
                if origin.is_some() || bed.is_some() {
                    return Err("--origin and --bed are options of the G-code and HP-GL outputs, not of a drawing".into());
                }
                let (text, left_out) = dxf_out::to_dxf(&shown, force)?;
                if left_out > 0 {
                    eprintln!("note: {left_out} obround opening(s) are not in the drawing");
                }
                let path = std::path::Path::new(out);
                if overwrite {
                    write_atomic(path, &text)?;
                } else {
                    dxf_out::write_new(path, &text)?;
                }
                println!(
                    "{} openings drawn in {out} (millimetres, after the reduction)",
                    shown.len() - left_out
                );
                return Ok(());
            }
            if create || overwrite {
                return Err("--create and --overwrite are options of a DXF output".into());
            }
            let mut once = paths(&openings, &p)?;
            if let Some(o) = origin {
                move_to_origin(&mut once, o);
            }
            if let Some(b) = bed {
                if let Some(pt) = outside_bed(&once, b) {
                    return Err(format!(
                        "the point {:.3},{:.3} falls outside the bed {},{}; move the stencil with --origin",
                        pt.0, pt.1, b.0, b.1
                    ));
                }
            }
            let once_forms = forms(&openings);
            // every contour is cut `passes` times: all the contours once, then all of them again
            let all: Vec<_> = (0..passes).flat_map(|_| once.iter().cloned()).collect();
            let all_forms: Vec<_> = (0..passes)
                .flat_map(|_| once_forms.iter().copied())
                .collect();
            let is_plot = out.to_ascii_lowercase().ends_with(".plt");
            let estimate = if rapid > 0.0 && !is_plot {
                Some(estimate_time_with_forms(
                    &all, &all_forms, feed, laser_wait, &leads, rapid,
                )?)
            } else {
                None
            };
            let text = if is_plot {
                hpgl(&all)
            } else {
                let head = header(&Header {
                    generator: "eh3d-stencilcam",
                    version: env!("CARGO_PKG_VERSION"),
                    source: Some((
                        std::path::Path::new(&layer)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("layer"),
                        sha256_hex(&bytes),
                    )),
                    tools: "laser",
                });
                format!(
                    "{head}{}",
                    gcode_with_form_leads(&all, &all_forms, feed, smax as u32, laser_wait, &leads)?
                )
            };
            write_atomic(std::path::Path::new(out), &text)?;
            println!("{} openings written to {out}", once.len());
            if passes > 1 {
                println!("{passes} passes over every contour");
            }
            if let Some(t) = estimate {
                println!(
                    "estimated time: {} ({} cutting, {} waiting for the laser, {} moving); the acceleration of the machine is not counted",
                    duration_text(t.total_s()),
                    duration_text(t.cutting_s),
                    duration_text(t.waiting_s),
                    duration_text(t.rapid_s)
                );
            }
            if let Some(link) = &link {
                let (thickness, shrink) =
                    (p.thickness_mm.to_string(), p.shrink_percent.to_string());
                let (feed_text, smax_text, passes_text, count) = (
                    feed.to_string(),
                    smax.to_string(),
                    passes.to_string(),
                    once.len().to_string(),
                );
                keep(
                    link,
                    out,
                    server::media_type(out),
                    text.as_bytes(),
                    &[
                        ("command", "convert"),
                        ("thickness_mm", thickness.as_str()),
                        ("shrink_percent", shrink.as_str()),
                        ("feed_mm_min", feed_text.as_str()),
                        ("smax", smax_text.as_str()),
                        ("passes", passes_text.as_str()),
                        ("openings", count.as_str()),
                    ],
                )?;
            }
            Ok(())
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

/// `project info FILE` and `project convert FILE [--layer NAME] [--plt] [options of convert]`: the paste layers of the job of a `.eh3d` project and the stencils kept in the branch
/// `stencilcam/`. Each layer is turned into a stencil by `convert` itself, with the same options and the same refusals.
fn project(mut args: Vec<String>) -> Result<(), String> {
    const USAGE: &str = "project needs info or convert and a project (project info FILE.eh3d, project convert FILE.eh3d [--layer NAME] [--plt] [options of convert])";
    if args.len() < 2 {
        return Err(USAGE.into());
    }
    let what = args.remove(0);
    let file = std::path::PathBuf::from(args.remove(0));
    match what.as_str() {
        "info" => {
            if let Some(extra) = args.first() {
                return Err(format!("{extra}: unexpected argument (see help)"));
            }
            let (lines, state) = suite::describe(&eh3dfile::KIT, &file)?;
            for line in lines {
                println!("{line}");
            }
            if state.out_of_date {
                let why = suite::out_of_date_message(&eh3dfile::KIT);
                return Err(format!("{NOT_PASSED}{why}"));
            }
            Ok(())
        }
        "convert" => {
            let named = flag(&mut args, "--layer")?;
            let plt = switch(&mut args, "--plt");
            if args
                .iter()
                .any(|a| matches!(a.as_str(), "--create" | "--overwrite"))
            {
                return Err("--create and --overwrite are options of a DXF output".into());
            }
            let layers = eh3dfile::paste_layers(&file)?;
            let chosen = match &named {
                Some(name) => vec![eh3dfile::choose(&layers, name)?.clone()],
                None => layers,
            };
            // every stencil is made before one is kept, so a refusal leaves the project as it was
            let mut made = Vec::new();
            for layer in chosen {
                let name = eh3dfile::result_name(&layer, if plt { "plt" } else { "nc" });
                let dir = eh3dfile::scratch_dir()?;
                let input = dir.join(layer.file_name());
                let output = dir.join(&name);
                let bytes = fs::write(&input, &layer.bytes)
                    .map_err(|e| format!("{}: {e}", input.display()))
                    .and_then(|()| {
                        let mut line = vec![
                            "convert".to_string(),
                            input.to_string_lossy().into_owned(),
                            output.to_string_lossy().into_owned(),
                        ];
                        line.extend(args.iter().cloned());
                        run(line)
                    })
                    .and_then(|()| {
                        fs::read(&output).map_err(|e| format!("{}: {e}", output.display()))
                    });
                let _ = fs::remove_dir_all(&dir);
                made.push((layer, name, bytes?));
            }
            for (layer, name, bytes) in made {
                for notice in eh3dfile::save_stencil(&file, &layer, &name, bytes)? {
                    eprintln!("note: {notice}");
                }
                println!(
                    "{name} kept in the branch {} of {}",
                    eh3dfile::KIT.branch,
                    file.display()
                );
            }
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

/// Whether the program opens its window instead of running a command, and the layer to open in it: no arguments, `gui` (with a layer or without), or a single layer file given by the
/// system to open (a name with an extension that is a file; the commands have none).
#[cfg(feature = "gui")]
fn window_request(args: &[String]) -> Option<Option<String>> {
    match args {
        [] => Some(None),
        [a] if a == "gui" => Some(None),
        [a, f] if a == "gui" => Some(Some(f.clone())),
        [f] if !f.starts_with('-')
            && std::path::Path::new(f).extension().is_some()
            && std::path::Path::new(f).is_file() =>
        {
            Some(Some(f.clone()))
        }
        _ => None,
    }
}

#[cfg(feature = "gui")]
fn open_window(layer: Option<String>) -> ExitCode {
    let lang = eh3d_stencilcam::lang::Lang::from_env();
    match eh3d_stencilcam::gui::run(layer.map(std::path::PathBuf::from), lang) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the window could not be opened: {e}");
            ExitCode::from(1)
        }
    }
}

/// The picture mode: `--shot FILE.png` opens the window, saves what it draws as a PNG file and closes. The exit code is 0 when the file was written, 1 when it was not and 2 when the
/// arguments are wrong.
#[cfg(feature = "gui")]
fn take_picture(args: &[String]) -> ExitCode {
    let parsed = match eh3d_stencilcam::shot::parse_window_args(args, 1) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(request) = parsed.shot else {
        eprintln!("error: --shot needs a file to write");
        return ExitCode::from(2);
    };
    let lang = parsed
        .lang
        .unwrap_or_else(eh3d_stencilcam::lang::Lang::from_env);
    match eh3d_stencilcam::gui::run_shot(parsed.files, lang, request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the picture could not be taken: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if eh3d_stencilcam::shot::wants_picture(&args) {
        #[cfg(feature = "gui")]
        return take_picture(&args);
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("error: this build has no window: build it with --features gui");
            return ExitCode::from(1);
        }
    }
    #[cfg(feature = "gui")]
    if let Some(layer) = window_request(&args) {
        return open_window(layer);
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => match e.strip_prefix(NOT_PASSED) {
            Some(text) => {
                eprintln!("error: {text}");
                ExitCode::from(2)
            }
            None => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
    }
}
