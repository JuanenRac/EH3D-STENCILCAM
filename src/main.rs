// EH3D-STENCILCAM - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_parser_gcode::emit::{header, sha256_hex, Header};
use eh3d_stencilcam::layer::{hidden_codes, read_layer, read_layer_file};
use eh3d_stencilcam::openings::{from_gerber_with, mirrored, ratios, shrunk, weak};
use eh3d_stencilcam::output::{gcode, hpgl, paths};
use eh3d_stencilcam::profile::{Material, Profile};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-stencilcam: solder paste stencils from the paste layer of a board

  eh3d-stencilcam convert LAYER.gtp OUT.nc|OUT.plt [--material mylar|kapton|steel] [--thickness MM] [--shrink PERCENT] [--radius MM]
                  [--kerf MM] [--feed 600] [--smax 1000] [--mirror] [--force]
  eh3d-stencilcam check LAYER.gtp [--material ...] [--thickness MM] [--shrink PERCENT]
  eh3d-stencilcam [gui] [LAYER.gtp]  the window (a build with --features gui): with no arguments, or with a single layer, it opens; the commands above never open it

Both read the layer with a size limit (--max-bytes N, 268435456 by default; the refusal is written in the language of the system) and show what the reader says about the file, each notice with its code; --hide CODE[,CODE] leaves
out the notices of those codes (for example units-missing). --arc-tolerance MM (0.005 by default) is how closely the arcs of the layer are followed.

OUT ending in .plt is HP-GL for a plotter; anything else is G-code for a laser. convert refuses to write when an opening cannot release the paste (aspect ratio under 1.5 or
area ratio under 0.66), or when the layer has pads, tracks or regions that cannot become openings, unless --force is given. --mirror is for the stencil of the bottom side.";

fn profile(args: &mut Vec<String>) -> Result<Profile, String> {
    let mut p = Profile::of(match flag(args, "--material")? {
        Some(m) => Material::parse(&m)?,
        None => Material::Mylar,
    });
    if let Some(v) = flag(args, "--thickness")? {
        p.thickness_mm = num(&v, "--thickness")?;
    }
    if let Some(v) = flag(args, "--shrink")? {
        p.shrink_percent = num(&v, "--shrink")?;
    }
    if let Some(v) = flag(args, "--radius")? {
        p.corner_radius_mm = num(&v, "--radius")?;
    }
    if let Some(v) = flag(args, "--kerf")? {
        p.kerf_mm = num(&v, "--kerf")?;
    }
    p.validate()?;
    Ok(p)
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
    let p = profile(&mut args)?;
    let mirror = switch(&mut args, "--mirror");
    let force = switch(&mut args, "--force");
    let feed = flag(&mut args, "--feed")?
        .map(|v| num(&v, "--feed"))
        .transpose()?
        .unwrap_or(600.0);
    let smax = flag(&mut args, "--smax")?
        .map(|v| num(&v, "--smax"))
        .transpose()?
        .unwrap_or(1000.0);
    if !(1.0..=100_000.0).contains(&smax) {
        return Err(format!("--smax {smax} must be from 1 to 100000"));
    }
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
    let reading = read_layer(&bytes, max_bytes, &hide).map_err(|e| format!("{layer}: {e}"))?;
    let (mut openings, mut notes) = from_gerber_with(&reading.file, arc_tolerance);
    if mirror {
        openings = mirrored(&openings);
    }
    for n in &notes {
        eprintln!("note: {n}");
    }
    for n in &reading.shown {
        eprintln!("note: {n}");
    }
    // what the reader lost is lost from the stencil too
    for _ in 0..reading.errors {
        notes.push(String::from("reader error"));
    }
    // a stencil that leaves out pads is not the stencil of the board: it is written only when it is asked for
    if cmd == "convert" && !notes.is_empty() && !force {
        return Err(format!(
            "{} part(s) of the layer cannot become openings (see the notes above): the stencil would be incomplete; use --force to write it anyway",
            notes.len()
        ));
    }
    let shown: Vec<_> = openings
        .iter()
        .map(|o| shrunk(o, &p))
        .collect::<Result<_, _>>()?;
    let problems = weak(&shown, p.thickness_mm)?;
    match cmd.as_str() {
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
                println!("smallest aspect ratio {a:.2} (at least 1.5), smallest area ratio {:.2} (at least 0.66)", all.iter().map(|r| r.area).fold(f64::INFINITY, f64::min));
            }
            for (_, m) in &problems {
                println!("{m}");
            }
            if problems.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "{} openings will not release the paste",
                    problems.len()
                ))
            }
        }
        "convert" => {
            let out = args.get(1).ok_or("convert needs an output file")?;
            if !problems.is_empty() && !force {
                return Err(format!("{} openings will not release the paste (run check to see them, or --force):\n{}", problems.len(), problems.iter().map(|(_, m)| m.as_str()).collect::<Vec<_>>().join("\n")));
            }
            let all = paths(&openings, &p)?;
            let text = if out.to_ascii_lowercase().ends_with(".plt") {
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
                format!("{head}{}", gcode(&all, feed, smax as u32)?)
            };
            fs::write(out, text).map_err(|e| format!("{out}: {e}"))?;
            println!("{} openings written to {out}", all.len());
            Ok(())
        }
        other => Err(format!("{other}: unknown command (see help)")),
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

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    #[cfg(feature = "gui")]
    if let Some(layer) = window_request(&args) {
        return open_window(layer);
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
