//! The G-code that the program writes is read back with the reader of the suite and walked the way the G-code debugger walks it in its strict mode: nothing in it may be a warning of
//! the reader, a rapid move into the material or a cut with the laser off.

use eh3d_parser_gcode::{interpret, parse, Dialect, Motion, Severity};
use eh3d_stencilcam::output::{gcode, gcode_with_wait};

/// What the strict mode of the debugger would report for `text`.
fn alerts(text: &str) -> Vec<String> {
    let trace = interpret(&parse(text, Dialect::Grbl));
    let mut found: Vec<String> = trace
        .diagnostics
        .iter()
        .filter(|d| d.severity >= Severity::Warning)
        .map(|d| format!("line {}: {}", d.line, d.message))
        .collect();
    for s in &trace.segments {
        let rapid = matches!(s.motion, Motion::Rapid);
        if !rapid && !s.spindle_on {
            found.push(format!("line {}: it cuts with the laser off", s.line));
        }
        if !rapid && s.feed <= 0.0 {
            found.push(format!("line {}: a cutting move has no feed", s.line));
        }
    }
    if !trace.ended {
        found.push("the program does not end".into());
    }
    found
}

fn square(x: f64, y: f64, side: f64) -> Vec<(f64, f64)> {
    vec![
        (x, y),
        (x + side, y),
        (x + side, y + side),
        (x, y + side),
        (x, y),
    ]
}

#[test]
fn the_checker_used_here_does_find_a_cut_with_the_laser_off() {
    assert!(!alerts("G21\nG90\nF600\nG1 X5\nM2\n").is_empty());
    assert!(!alerts("G21\nG90\nM3 S100\nG1 X5\nM2\n").is_empty());
    assert!(alerts("G21\nG90\nF600\nM3 S100\nG1 X5\nM5\nM2\n").is_empty());
}

#[test]
fn the_gcode_of_the_cuts_has_no_alert_with_or_without_a_wait() {
    let paths = vec![
        square(0.0, 0.0, 1.0),
        square(5.0, 5.0, 2.0),
        square(9.0, 0.0, 1.0),
    ];
    for wait in [0.0, 0.5] {
        let g = gcode_with_wait(&paths, 600.0, 1000, wait).unwrap();
        let found = alerts(&g);
        assert!(found.is_empty(), "{wait}: {found:?}\n{g}");
    }
    let plain = gcode(&paths, 600.0, 1000).unwrap();
    // three paths of four sides each are at least twelve cutting moves, all of them with the laser on
    let trace = interpret(&parse(&plain, Dialect::Grbl));
    let cuts = trace
        .segments
        .iter()
        .filter(|s| matches!(s.motion, Motion::Linear))
        .count();
    assert!(cuts >= 12, "{cuts} cutting moves\n{plain}");
}

#[test]
fn the_command_line_writes_the_header_first_and_the_program_has_no_alert() {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_strict_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let layer = dir.join("paste.gtp");
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";
    std::fs::write(&layer, text).unwrap();
    let out = dir.join("out.nc");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args([
            "convert",
            layer.to_str().unwrap(),
            out.to_str().unwrap(),
            "--thickness",
            "0.1",
            "--laser-wait",
            "0.2",
        ])
        .output()
        .expect("the program starts");
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let g = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        g.starts_with(&format!(
            "; generator: eh3d-stencilcam {}\n; units: mm (G21)\n; source: paste.gtp sha256:",
            env!("CARGO_PKG_VERSION")
        )),
        "{g}"
    );
    assert!(g.contains("G4 P0.2\n"), "{g}");
    let found = alerts(&g);
    assert!(found.is_empty(), "{found:?}\n{g}");
}
