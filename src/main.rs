// EH3D-STENCILCAM - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_stencilcam::openings::{from_gerber, mirrored, ratios, shrunk, weak};
use eh3d_stencilcam::output::{gcode, hpgl, paths};
use eh3d_stencilcam::profile::{Material, Profile};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-stencilcam: solder paste stencils from the paste layer of a board

  eh3d-stencilcam convert LAYER.gtp OUT.nc|OUT.plt [--material mylar|kapton|steel] [--thickness MM] [--shrink PERCENT] [--radius MM]
                  [--kerf MM] [--feed 600] [--smax 1000] [--mirror] [--force]
  eh3d-stencilcam check LAYER.gtp [--material ...] [--thickness MM] [--shrink PERCENT]

OUT ending in .plt is HP-GL for a plotter; anything else is G-code for a laser. convert refuses to write when an opening cannot release the paste (aspect ratio under 1.5 or
area ratio under 0.66) unless --force is given. --mirror is for the stencil of the bottom side.";

fn flag(args: &mut Vec<String>, name: &str) -> Result<Option<String>, String> {
    if let Some(i) = args.iter().position(|a| a == name) {
        let v = args
            .get(i + 1)
            .ok_or_else(|| format!("{name} needs a value"))?
            .clone();
        args.drain(i..=i + 1);
        return Ok(Some(v));
    }
    Ok(None)
}

fn switch(args: &mut Vec<String>, name: &str) -> bool {
    if let Some(i) = args.iter().position(|a| a == name) {
        args.remove(i);
        true
    } else {
        false
    }
}

fn num(text: &str, what: &str) -> Result<f64, String> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{what} '{text}' is not a number"))
}

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
    let layer = args
        .first()
        .ok_or_else(|| format!("{cmd} needs a paste layer"))?
        .clone();
    let bytes = fs::read(&layer).map_err(|e| format!("{layer}: {e}"))?;
    let (mut openings, notes) =
        from_gerber(&eh3d_parser_gerber::parse(&String::from_utf8_lossy(&bytes)));
    if mirror {
        openings = mirrored(&openings);
    }
    for n in &notes {
        eprintln!("note: {n}");
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
                gcode(&all, feed, smax as u32)?
            };
            fs::write(out, text).map_err(|e| format!("{out}: {e}"))?;
            println!("{} openings written to {out}", all.len());
            Ok(())
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
