//! The sheets of stencil material from the table of sheets that the tool crib exports: the reader (columns by name, limits, faults that name their row, a fuzz with a fixed seed), the
//! options `--sheet-file` and `--sheet` and their keys in the settings file, which of the sources wins, and the window that shows the sheet that was chosen.

use eh3d_stencilcam::config::{parse as parse_settings, sheet_choice};
use eh3d_stencilcam::crib::{
    sheet_from_crib, sheet_from_file, sheets_from_crib, SheetSpec, MAX_CRIB_BYTES, MAX_CRIB_ROWS,
};
use eh3d_stencilcam::lang::Lang;
use eh3d_stencilcam::openings::{shrunk, Form, Limits, Opening};
use eh3d_stencilcam::profile::{Material, Profile};
use eh3d_stencilcam::window::{Status, Window};
use std::fs;
use std::process::Command;

const HEADER: &str =
    "id,name,kind,material,thickness_mm,reduction_mm,kerf_mm,min_aspect_ratio,min_area_ratio";

fn table(rows: &[&str]) -> String {
    let mut text = String::from(HEADER);
    text.push('\n');
    for r in rows {
        text.push_str(r);
        text.push('\n');
    }
    text
}

const SHEETS: [&str; 4] = [
    "1,Mylar 0.1,sheet,Mylar,0.1,0.05,0.08,0,0",
    "2,\"Kapton, thin\",sheet,Kapton,0.075,0,0,2,0.5",
    "3,Steel,sheet,,0.12,0.02,0,0,0",
    "4,Strict,sheet,Mylar,0.1,0,0.08,4.9,1.2",
];

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("eh3d_stencil_sheets_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_sheets_are_read_by_the_names_of_their_columns_in_any_order() {
    let sheets = sheets_from_crib(&table(&SHEETS)).unwrap();
    assert_eq!(sheets.len(), 4);
    assert_eq!(sheets[0].name, "Mylar 0.1");
    assert_eq!(
        (
            sheets[0].thickness_mm,
            sheets[0].reduction_mm,
            sheets[0].kerf_mm
        ),
        (0.1, 0.05, 0.08)
    );
    assert_eq!(sheets[1].name, "Kapton, thin");
    assert_eq!(
        (sheets[1].min_aspect_ratio, sheets[1].min_area_ratio),
        (2.0, 0.5)
    );
    assert_eq!(sheets[2].material, "");
    // any case and spaces around when the sheet is looked up
    assert_eq!(
        sheet_from_crib(&table(&SHEETS), "  kapton, THIN ")
            .unwrap()
            .material,
        "Kapton"
    );
    // another order of the columns, a byte order mark and the line ends of a spreadsheet
    let other = "\u{feff}min_area_ratio,kerf_mm,name,kind,material,reduction_mm,thickness_mm,min_aspect_ratio\r\n0.3,0.04,\"Say \"\"hi\"\"\",sheet,Steel,0.01,0.2,1.5\r\n";
    let s = sheet_from_crib(other, "say \"hi\"").unwrap();
    assert_eq!(
        (
            s.thickness_mm,
            s.reduction_mm,
            s.kerf_mm,
            s.min_aspect_ratio,
            s.min_area_ratio
        ),
        (0.2, 0.01, 0.04, 1.5, 0.3)
    );
    // a name the crib marked against a formula is given back as it was written
    let marked = table(&["1,'=cmd,sheet,'+x,0.1,0,0,0,0"]);
    let s = &sheets_from_crib(&marked).unwrap()[0];
    assert_eq!((s.name.as_str(), s.material.as_str()), ("=cmd", "+x"));
    assert!(sheet_from_crib(&table(&SHEETS), "Nothing")
        .unwrap_err()
        .contains("no sheet called 'Nothing'"));
}

#[test]
fn every_fault_of_a_table_of_sheets_is_an_error_that_says_its_row() {
    let row = |cells: &str| table(&[SHEETS[0], cells]);
    for (text, why) in [
        (String::new(), "the crib is empty"),
        (
            "id,name,kind,diameter_mm\n1,a,laser,0.1\n".to_string(),
            "no 'material' column",
        ),
        (HEADER.replace(",kerf_mm", "") + "\n", "no 'kerf_mm' column"),
        (
            "name,kind,diameter_mm,material\n".to_string(),
            "no 'thickness_mm' column: it is not a table of sheets",
        ),
        (
            row("2,B,drill,,0.1,0,0,0,0"),
            "row 3 of the crib: the kind is 'drill', not 'sheet'",
        ),
        (row("2,B,sheet"), "row 3 of the crib has 3 cells"),
        (
            row("2,mylar 0.1,sheet,,0.1,0,0,0,0"),
            "row 3 of the crib: the sheet 'mylar 0.1' is already in the table",
        ),
        (
            row("2,,sheet,,0.1,0,0,0,0"),
            "row 3 of the crib: the name of a sheet must have text",
        ),
        (
            row("2,B,sheet,,thin,0,0,0,0"),
            "row 3 of the crib: thickness_mm of the sheet 'B' is not a number",
        ),
        (
            row("2,B,sheet,,0.1,0,NaN,0,0"),
            "row 3 of the crib: kerf_mm of the sheet 'B' is not a number",
        ),
        (
            row("2,B,sheet,,0,0,0,0,0"),
            "thickness_mm of the sheet 'B' is 0: it must be more than 0 and at most 50",
        ),
        (row("2,B,sheet,,51,0,0,0,0"), "is 51"),
        (
            row("2,B,sheet,,0.1,0.6,0,0,0"),
            "reduction_mm of the sheet 'B' is 0.6: it must be from 0 to 0.5",
        ),
        (row("2,B,sheet,,0.1,-0.1,0,0,0"), "is -0.1"),
        (
            row("2,B,sheet,,0.1,0,1.5,0,0"),
            "kerf_mm of the sheet 'B' is 1.5",
        ),
        (
            row("2,B,sheet,,0.1,0,0,11,0"),
            "min_aspect_ratio of the sheet 'B' is 11",
        ),
        (
            row("2,B,sheet,,0.1,0,0,0,-1"),
            "min_area_ratio of the sheet 'B' is -1",
        ),
        (row("2,B,sheet,,0.1,0,0,inf,0"), "not a number"),
        (row("2,\"B,sheet,,0.1,0,0,0,0"), "never closed"),
    ] {
        let e = sheets_from_crib(&text).unwrap_err();
        assert!(e.contains(why), "{text:?}: {e}");
    }
}

#[test]
fn a_table_over_the_limits_is_refused_and_a_file_is_read_with_the_same_limits() {
    let mut t = String::from(HEADER);
    t.push('\n');
    for i in 0..=MAX_CRIB_ROWS + 1 {
        t.push_str(&format!("{i},S{i},sheet,,0.1,0,0,0,0\n"));
    }
    assert!(sheets_from_crib(&t).unwrap_err().contains("more than"));
    let dir = scratch("limits");
    let big = dir.join("big.csv");
    fs::write(&big, vec![b'a'; MAX_CRIB_BYTES as usize + 1]).unwrap();
    assert!(sheet_from_file(big.to_str().unwrap(), "x")
        .unwrap_err()
        .contains("over the limit"));
    let binary = dir.join("bin.csv");
    fs::write(&binary, [0xffu8, 0xfe, 0x00]).unwrap();
    assert!(sheet_from_file(binary.to_str().unwrap(), "x")
        .unwrap_err()
        .contains("not UTF-8"));
    assert!(sheet_from_file(dir.join("none.csv").to_str().unwrap(), "x").is_err());
    let good = dir.join("sheets.csv");
    fs::write(&good, table(&SHEETS)).unwrap();
    let path = good.to_str().unwrap();
    assert_eq!(sheet_from_file(path, "steel").unwrap().thickness_mm, 0.12);
    // a fault names the file as well as the row
    fs::write(&good, table(&["1,A,sheet,,0,0,0,0,0"])).unwrap();
    let e = sheet_from_file(path, "A").unwrap_err();
    assert!(e.starts_with(path) && e.contains("row 2"), "{e}");
    let _ = fs::remove_dir_all(&dir);
}

struct Seed(u64);

impl Seed {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

#[test]
fn a_damaged_table_of_sheets_is_read_without_a_panic_and_every_sheet_that_comes_out_is_in_range() {
    let base = table(&SHEETS).into_bytes();
    let mut s = Seed(0x5EED_0031);
    for round in 0..600 {
        let mut bytes = base.clone();
        for _ in 0..1 + s.below(8) {
            let at = s.below(bytes.len() as u64) as usize;
            match s.below(4) {
                0 => bytes[at] = s.below(256) as u8,
                1 => {
                    bytes.remove(at);
                }
                2 => bytes.insert(at, b"\",\n'=\r-"[s.below(7) as usize]),
                _ => bytes.truncate(at),
            }
            if bytes.is_empty() {
                break;
            }
        }
        let text = String::from_utf8_lossy(&bytes);
        if let Ok(sheets) = sheets_from_crib(&text) {
            for sh in &sheets {
                assert!(
                    sh.thickness_mm > 0.0 && sh.thickness_mm <= 50.0,
                    "round {round}: {sh:?}"
                );
                assert!((0.0..=0.5).contains(&sh.reduction_mm), "round {round}");
                assert!((0.0..=1.0).contains(&sh.kerf_mm), "round {round}");
                assert!((0.0..=10.0).contains(&sh.min_aspect_ratio), "round {round}");
                assert!((0.0..=10.0).contains(&sh.min_area_ratio), "round {round}");
                assert!(!sh.name.is_empty(), "round {round}");
            }
            if let Some(first) = sheets.first() {
                assert_eq!(&sheet_from_crib(&text, &first.name).unwrap(), first);
            }
        }
    }
}

fn spec(
    name: &str,
    thickness: f64,
    reduction: f64,
    kerf: f64,
    aspect: f64,
    area: f64,
) -> SheetSpec {
    SheetSpec {
        name: name.into(),
        material: "Mylar".into(),
        thickness_mm: thickness,
        reduction_mm: reduction,
        kerf_mm: kerf,
        min_aspect_ratio: aspect,
        min_area_ratio: area,
    }
}

#[test]
fn a_sheet_sets_the_profile_and_its_limits_replace_the_base_only_when_they_are_not_0() {
    let mut p = Profile::of(Material::Mylar);
    let s = spec("A", 0.2, 0.03, 0.0, 0.0, 0.0);
    s.apply(&mut p);
    assert_eq!(
        (p.thickness_mm, p.shrink_percent, p.reduction_mm),
        (0.2, 0.0, 0.03)
    );
    let base = Limits {
        min_aspect: 1.2,
        min_area: 0.5,
    };
    assert_eq!(s.limits(base).unwrap(), base);
    let l = spec("B", 0.1, 0.0, 0.0, 3.0, 0.0).limits(base).unwrap();
    assert_eq!((l.min_aspect, l.min_area), (3.0, 0.5));
    let l = spec("C", 0.1, 0.0, 0.0, 0.0, 1.5).limits(base).unwrap();
    assert_eq!((l.min_aspect, l.min_area), (1.2, 1.5));
    // a figure this program does not take is an error that names the sheet
    let e = spec("D", 0.1, 0.0, 0.0, 0.0, 3.0).limits(base).unwrap_err();
    assert!(e.contains("'D'") && e.contains("min_area_ratio 3"), "{e}");
    let e = spec("E", 0.1, 0.0, 0.0, 0.2, 0.0).limits(base).unwrap_err();
    assert!(e.contains("min_aspect_ratio 0.2"), "{e}");
    // the reduction takes the opening smaller on each side; one it would leave with no size is an error
    let o = Opening {
        x: 1.0,
        y: 1.0,
        w: 1.0,
        h: 0.5,
        form: Form::Rectangle,
    };
    let mut p = Profile::of(Material::Mylar);
    spec("F", 0.1, 0.05, 0.0, 0.0, 0.0).apply(&mut p);
    let r = shrunk(&o, &p).unwrap();
    assert!(
        (r.w - 0.9).abs() < 1e-12 && (r.h - 0.4).abs() < 1e-12,
        "{r:?}"
    );
    p.reduction_mm = 0.25;
    assert!(shrunk(&o, &p)
        .unwrap_err()
        .contains("is gone after the reduction"));
    p.reduction_mm = 0.6;
    assert!(shrunk(&o, &p).is_err());
    assert!(spec("G", 0.1, 0.05, 0.08, 0.0, 0.0)
        .summary()
        .contains("kerf 0.08 mm"));
    assert!(spec("G", 0.1, 0.05, 0.0, 0.0, 0.0)
        .summary()
        .contains("from a laser of the crib"));
}

#[test]
fn the_settings_file_chooses_the_sheet_with_two_keys_and_a_bad_line_says_its_number() {
    let text = "# a sheet\nsheet_file = my sheets.csv\nsheet_name = Mylar 0.1 # the name\n";
    assert!(parse_settings(text).is_ok());
    assert_eq!(
        sheet_choice(text),
        (
            Some("my sheets.csv".to_string()),
            Some("Mylar 0.1".to_string())
        )
    );
    assert_eq!(sheet_choice("feed_mm_min = 300\n"), (None, None));
    assert_eq!(
        sheet_choice("sheet_name = x\n"),
        (None, Some("x".to_string()))
    );
    let e = parse_settings("feed_mm_min = 300\nsheet_file =\n").unwrap_err();
    assert!(
        e.contains("line 2") && e.contains("sheet_file has no value"),
        "{e}"
    );
    let e = parse_settings("sheet_name = a\nsheet_name = b\n").unwrap_err();
    assert!(e.contains("line 2") && e.contains("given twice"), "{e}");
    // the default file has the keys as comments, so it is still the default
    assert!(eh3d_stencilcam::config::default_file().contains("# sheet_file ="));
}

const PADS: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";
/// The same pads already made smaller by 0.05 mm on each side.
const REDUCED_PADS: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,0.9X0.4*%\nD10*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";
const CRIB: &str = "id,name,kind,diameter_mm\n1,Fibre,laser,0.08\n";

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn cuts(path: &std::path::Path) -> String {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with('G') || l.starts_with('M'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_options_take_the_thickness_the_reduction_and_the_kerf_of_the_sheet() {
    let dir = scratch("cli");
    let (layer, reduced, sheets, crib) = (
        dir.join("paste.gtp"),
        dir.join("reduced.gtp"),
        dir.join("sheets.csv"),
        dir.join("crib.csv"),
    );
    fs::write(&layer, PADS).unwrap();
    fs::write(&reduced, REDUCED_PADS).unwrap();
    fs::write(&sheets, table(&SHEETS)).unwrap();
    fs::write(&crib, CRIB).unwrap();
    let (l, r, sh, c) = (
        layer.to_str().unwrap(),
        reduced.to_str().unwrap(),
        sheets.to_str().unwrap(),
        crib.to_str().unwrap(),
    );
    let (a, b) = (dir.join("a.nc"), dir.join("b.nc"));
    let (pa, pb) = (a.to_str().unwrap(), b.to_str().unwrap());
    // the sheet 'Mylar 0.1' (0.1 mm, 0.05 mm less on each side, kerf 0.08) is the same as the pads already reduced with the same thickness and kerf by hand
    let (code, out, why) = run(&["convert", l, pa, "--sheet-file", sh, "--sheet", "mylar 0.1"]);
    assert_eq!(code, Some(0), "{why}");
    assert!(
        out.contains(
            "sheet 'Mylar 0.1' (Mylar): thickness 0.1 mm, reduction 0.05 mm per side, kerf 0.08 mm"
        ),
        "{out}"
    );
    let (code, _, why) = run(&[
        "convert",
        r,
        pb,
        "--thickness",
        "0.1",
        "--shrink",
        "0",
        "--kerf",
        "0.08",
    ]);
    assert_eq!(code, Some(0), "{why}");
    assert_eq!(cuts(&a), cuts(&b));
    // a sheet with a kerf of 0 takes the kerf of the laser of the crib, or --kerf, and nothing else
    let (c1, c2) = (dir.join("c1.nc"), dir.join("c2.nc"));
    let (pc1, pc2) = (c1.to_str().unwrap(), c2.to_str().unwrap());
    let (code, _, why) = run(&[
        "convert",
        l,
        pc1,
        "--sheet-file",
        sh,
        "--sheet",
        "Steel",
        "--crib",
        c,
        "--crib-laser",
        "fibre",
    ]);
    assert_eq!(code, Some(0), "{why}");
    let (code, _, why) = run(&[
        "convert",
        l,
        pc2,
        "--sheet-file",
        sh,
        "--sheet",
        "Steel",
        "--kerf",
        "0.08",
    ]);
    assert_eq!(code, Some(0), "{why}");
    assert_eq!(cuts(&c1), cuts(&c2));
    let (code, _, why) = run(&["check", l, "--sheet-file", sh, "--sheet", "Steel"]);
    assert_eq!(code, Some(1));
    assert!(
        why.contains("takes its kerf from a laser of the crib"),
        "{why}"
    );
    // what the sheet gives is not given twice
    for (extra, text) in [
        (vec!["--kerf", "0.1"], "has a kerf of its own"),
        (
            vec!["--crib", c, "--crib-laser", "Fibre"],
            "has a kerf of its own",
        ),
        (vec!["--thickness", "0.1"], "do not give --thickness too"),
        (vec!["--shrink", "5"], "do not give --shrink too"),
    ] {
        let mut args = vec!["check", l, "--sheet-file", sh, "--sheet", "Mylar 0.1"];
        args.extend(extra);
        let (code, _, why) = run(&args);
        assert_eq!(code, Some(1), "{args:?}");
        assert!(why.contains(text), "{why}");
    }
    // the file and the name go together, and a sheet that is not in the table is an error
    let (code, _, why) = run(&["check", l, "--sheet", "Steel"]);
    assert_eq!(code, Some(1));
    assert!(why.contains("go together"), "{why}");
    let (code, _, why) = run(&["check", l, "--sheet-file", sh, "--sheet", "Nothing"]);
    assert_eq!(code, Some(1));
    assert!(
        why.contains("no sheet called 'Nothing'") && why.contains("sheets.csv"),
        "{why}"
    );
    // a sheet too thick for this program says which sheet
    fs::write(&sheets, table(&["1,Thick,sheet,,5,0,0.05,0,0"])).unwrap();
    let (code, _, why) = run(&["check", l, "--sheet-file", sh, "--sheet", "Thick"]);
    assert_eq!(code, Some(1));
    assert!(why.contains("sheet 'Thick': thickness_mm is 5"), "{why}");
    // a reduction that leaves an opening with no size is a mistake
    fs::write(&sheets, table(&["1,Cut,sheet,,0.1,0.25,0.05,0,0"])).unwrap();
    let (code, _, why) = run(&["check", l, "--sheet-file", sh, "--sheet", "Cut"]);
    assert_eq!(code, Some(1));
    assert!(why.contains("is gone after the reduction"), "{why}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_command_line_wins_over_the_sheet_and_the_sheet_over_the_settings_file() {
    let dir = scratch("winlimits");
    let (layer, sheets, cfg) = (
        dir.join("paste.gtp"),
        dir.join("sheets.csv"),
        dir.join("stencil.cfg"),
    );
    fs::write(&layer, PADS).unwrap();
    fs::write(&sheets, table(&SHEETS)).unwrap();
    let (l, sh, cf) = (
        layer.to_str().unwrap(),
        sheets.to_str().unwrap(),
        cfg.to_str().unwrap(),
    );
    // the pads are 1.0 x 0.5 mm: on 0.1 mm the narrow side is 5 times the thickness, and the sheet 'Strict' asks for 4.9 and an area ratio of 1.2
    let strict = ["--sheet-file", sh, "--sheet", "Strict"];
    let with = |extra: &[&str]| {
        let mut args = vec!["check", l];
        args.extend(strict);
        args.extend(extra);
        run(&args)
    };
    let (code, _, why) = with(&[]);
    assert_eq!(
        code,
        Some(0),
        "aspect 5 and area 1.67 pass 4.9 and 1.2: {why}"
    );
    fs::write(&sheets, table(&["1,Strict,sheet,,0.1,0,0.08,6,0"])).unwrap();
    let (code, _, why) = with(&[]);
    assert_eq!(
        code,
        Some(2),
        "the aspect ratio 5 is below the 6 of the sheet"
    );
    assert!(why.contains("will not release the paste"), "{why}");
    // the command line wins
    assert_eq!(with(&["--min-aspect", "4"]).0, Some(0));
    // the sheet wins over the settings file
    fs::write(&cfg, "min_aspect = 4\n").unwrap();
    assert_eq!(with(&["--config", cf]).0, Some(2));
    assert_eq!(with(&["--config", cf, "--min-aspect", "4"]).0, Some(0));
    // a sheet that gives no limit leaves the one of the settings file
    fs::write(&sheets, table(&["1,Strict,sheet,,0.1,0,0.08,0,0"])).unwrap();
    fs::write(&cfg, "min_aspect = 6\n").unwrap();
    assert_eq!(with(&["--config", cf]).0, Some(2));
    assert_eq!(with(&[]).0, Some(0));
    // a limit this program does not take is a mistake that names the sheet
    fs::write(&sheets, table(&["1,Strict,sheet,,0.1,0,0.08,0,3"])).unwrap();
    let (code, _, why) = with(&[]);
    assert_eq!(code, Some(1));
    assert!(
        why.contains("'Strict'") && why.contains("min_area_ratio 3"),
        "{why}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_settings_file_chooses_the_sheet_and_the_options_choose_over_it() {
    let dir = scratch("cfg");
    let (layer, sheets, cfg) = (
        dir.join("paste.gtp"),
        dir.join("sheets.csv"),
        dir.join("stencil.cfg"),
    );
    fs::write(&layer, PADS).unwrap();
    fs::write(&sheets, table(&SHEETS)).unwrap();
    let (l, sh, cf) = (
        layer.to_str().unwrap(),
        sheets.to_str().unwrap(),
        cfg.to_str().unwrap(),
    );
    fs::write(&cfg, format!("sheet_file = {sh}\nsheet_name = Mylar 0.1\n")).unwrap();
    let (code, out, why) = run(&["check", l, "--config", cf]);
    assert_eq!(code, Some(0), "{why}");
    assert!(out.contains("sheet 'Mylar 0.1'"), "{out}");
    // the option names another sheet of the same file; the thickness changes with it
    let (code, out, why) = run(&["check", l, "--config", cf, "--sheet", "Kapton, thin"]);
    assert_eq!(code, Some(1), "{why}");
    assert!(
        why.contains("takes its kerf") || why.contains("kerf"),
        "{why}{out}"
    );
    let (code, out, _) = run(&[
        "check",
        l,
        "--config",
        cf,
        "--sheet",
        "Kapton, thin",
        "--kerf",
        "0.05",
    ]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("thickness 0.075 mm"), "{out}");
    // only one of the two keys is not enough
    fs::write(&cfg, format!("sheet_file = {sh}\n")).unwrap();
    let (code, _, why) = run(&["check", l, "--config", cf]);
    assert_eq!(code, Some(1));
    assert!(why.contains("go together"), "{why}");
    // check --json keeps the standard output as the JSON alone
    fs::write(&cfg, format!("sheet_file = {sh}\nsheet_name = Mylar 0.1\n")).unwrap();
    let (code, out, err) = run(&["check", l, "--config", cf, "--json"]);
    assert_eq!(code, Some(0));
    assert!(
        out.trim_start().starts_with('{') && !out.contains("sheet '"),
        "{out}"
    );
    assert!(err.contains("note: sheet 'Mylar 0.1'"), "{err}");
    let _ = fs::remove_dir_all(&dir);
}

fn layer_in(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join("paste.gtp");
    fs::write(&p, PADS).unwrap();
    p
}

#[test]
fn the_window_shows_the_sheet_that_was_chosen_in_the_language_of_the_person() {
    let dir = scratch("window");
    let layer = layer_in(&dir);
    let mut w = Window::new(Lang::En);
    assert_eq!(w.sheet_text(), None);
    assert!(w.open(&layer));
    assert!(w.set_sheet(Some(spec("Mylar 0.1", 0.1, 0.05, 0.08, 0.0, 0.0))));
    let text = |w: &Window, key: &str| w.fields.iter().find(|f| f.key == key).unwrap().text.clone();
    assert_eq!(
        (
            text(&w, "thickness_mm"),
            text(&w, "shrink_percent"),
            text(&w, "kerf_mm")
        ),
        ("0.1".to_string(), "0".to_string(), "0.08".to_string())
    );
    assert_eq!(
        w.sheet_text().unwrap(),
        "Sheet Mylar 0.1: thickness 0.1 mm, reduction 0.05 mm per side, kerf 0.08 mm"
    );
    // the openings are the pads reduced by 0.05 mm on each side: 0.9 x 0.4 on 0.1 mm
    let (aspect, _) = w.smallest_ratios().unwrap();
    assert!((aspect - 4.0).abs() < 1e-9, "{aspect}");
    assert_eq!(
        w.status,
        Status::Computed {
            openings: 2,
            weak: 0
        }
    );
    w.lang = Lang::Pt;
    assert_eq!(
        w.sheet_text().unwrap(),
        "Folha Mylar 0.1: espessura 0.1 mm, redução 0.05 mm por lado, largura de corte 0.08 mm"
    );
    w.lang = Lang::Ja;
    assert!(w.sheet_text().unwrap().starts_with("シート Mylar 0.1："));
    // a limit of the sheet moves what is said to be weak: an aspect ratio of 5 is more than the 4 that the openings have
    assert!(w.set_sheet(Some(spec("Strict", 0.1, 0.05, 0.08, 5.0, 0.0))));
    assert_eq!(w.limits().min_aspect, 5.0);
    assert_eq!(w.weak.len(), 2);
    assert_eq!(w.status, Status::Weak(2));
    // changing the material keeps the sheet over the values of the material
    w.set_material(Material::Steel);
    assert_eq!(text(&w, "thickness_mm"), "0.1");
    assert!(w.sheet_text().is_some());
    // a sheet whose limit the program cannot take is refused and the old one stays
    assert!(!w.set_sheet(Some(spec("Odd", 0.1, 0.0, 0.0, 0.0, 3.0))));
    assert!(matches!(&w.status, Status::Failed(e) if e.contains("'Odd'")));
    assert_eq!(w.sheet.as_ref().unwrap().name, "Strict");
    // no sheet: the values of the material come back
    assert!(w.set_sheet(None));
    assert_eq!(w.sheet_text(), None);
    assert_eq!(w.limits(), Limits::default());
    assert_ne!(text(&w, "thickness_mm"), "0.1");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_window_reads_a_sheet_from_the_file_with_the_limits_of_the_reader() {
    let dir = scratch("windowfile");
    let layer = layer_in(&dir);
    let sheets = dir.join("sheets.csv");
    fs::write(&sheets, table(&SHEETS)).unwrap();
    let mut w = Window::new(Lang::Es);
    assert!(w.open(&layer));
    assert!(w.load_sheet(sheets.to_str().unwrap(), "KAPTON, THIN"));
    assert_eq!(w.sheet.as_ref().unwrap().thickness_mm, 0.075);
    assert!(w
        .sheet_text()
        .unwrap()
        .starts_with("Lámina Kapton, thin: espesor 0.075 mm"));
    assert!(!w.load_sheet(sheets.to_str().unwrap(), "Nothing"));
    assert!(matches!(&w.status, Status::Failed(e) if e.contains("no sheet called")));
    assert_eq!(
        w.sheet.as_ref().unwrap().name,
        "Kapton, thin",
        "the old sheet stays"
    );
    assert!(!w.load_sheet(dir.join("none.csv").to_str().unwrap(), "x"));
    let _ = fs::remove_dir_all(&dir);
}
