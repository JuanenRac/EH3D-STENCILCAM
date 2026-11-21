//! The kerf of the laser from the CSV that the tool crib of the suite exports: a laser tool of the table gives the width of its beam. The file is read with limits, every fault names its
//! row, and nothing a damaged table holds ever panics.

use eh3d_stencilcam::crib::{
    kerf_from_crib, kerf_from_file, laser_tools, MAX_CRIB_BYTES, MAX_CRIB_ROWS,
};
use std::fs;
use std::process::Command;

const HEADER: &str = "id,name,kind,diameter_mm,flutes,length_mm,tip_angle_deg,shank_mm,material,hours,meters,holes,max_hours,max_holes";

fn table(rows: &[&str]) -> String {
    let mut text = String::from(HEADER);
    text.push('\n');
    for r in rows {
        text.push_str(r);
        text.push('\n');
    }
    text
}

const CRIB: [&str; 4] = [
    "1,Drill 0.8,drill,0.8,2,10,,3.175,carbide,0,0,0,0,0",
    "2,\"Fibre laser, 20 W\",laser,0.08,0,0,,0,,0,0,0,0,0",
    "3,Diode,LASER-not,0.2,0,0,,0,,0,0,0,0,0",
    "4,Diode 5 W,laser,0.12,0,0,,0,,0,0,0,0,0",
];

#[test]
fn the_kerf_is_the_beam_width_of_the_laser_that_is_named() {
    let t = table(&CRIB);
    let tools = laser_tools(&t).unwrap();
    assert_eq!(tools.len(), 2, "{tools:?}");
    assert_eq!(tools[0].name, "Fibre laser, 20 W");
    assert_eq!(kerf_from_crib(&t, "Fibre laser, 20 W").unwrap(), 0.08);
    assert_eq!(
        kerf_from_crib(&t, "  diode 5 w ").unwrap(),
        0.12,
        "any case, spaces around"
    );
    // a tool of another kind is not a laser, whatever its name
    assert!(kerf_from_crib(&t, "Drill 0.8")
        .unwrap_err()
        .contains("no laser called 'Drill 0.8'"));
    assert!(kerf_from_crib(&t, "Diode")
        .unwrap_err()
        .contains("no laser called"));
}

#[test]
fn a_table_with_the_columns_in_another_order_and_a_byte_order_mark_is_read() {
    let t = "\u{feff}kind,diameter_mm,name\r\nlaser,0.1,\"Say \"\"hi\"\"\"\r\ndrill,1,x\r\n";
    assert_eq!(kerf_from_crib(t, "say \"hi\"").unwrap(), 0.1);
}

#[test]
fn a_name_the_crib_marked_against_a_formula_is_given_back_as_it_was_written() {
    let t = table(&["1,'=cmd,laser,0.1,0,0,,0,,0,0,0,0,0"]);
    assert_eq!(laser_tools(&t).unwrap()[0].name, "=cmd");
    let t = table(&["1,'plain,laser,0.1,0,0,,0,,0,0,0,0,0"]);
    assert_eq!(laser_tools(&t).unwrap()[0].name, "'plain");
}

#[test]
fn every_fault_of_the_table_is_an_error_that_says_where() {
    for (text, why) in [
        (String::new(), "the crib is empty"),
        (
            "id,name,kind\n1,a,laser\n".to_string(),
            "no 'diameter_mm' column",
        ),
        (
            "name,kind,diameter_mm\nx,laser\n".to_string(),
            "row 2 of the crib has 2 cells",
        ),
        (
            "name,kind,diameter_mm\nx,laser,wide\n".to_string(),
            "row 2 of the crib: the diameter of the laser 'x' is not a number",
        ),
        (
            "name,kind,diameter_mm\nx,laser,NaN\n".to_string(),
            "not a number",
        ),
        (
            "name,kind,diameter_mm\n\"x,laser,0.1\n".to_string(),
            "never closed",
        ),
    ] {
        let e = laser_tools(&text).unwrap_err();
        assert!(e.contains(why), "{text:?}: {e}");
    }
    let two = table(&[
        "1,A,laser,0.1,0,0,,0,,0,0,0,0,0",
        "2,a,laser,0.2,0,0,,0,,0,0,0,0,0",
    ]);
    assert!(kerf_from_crib(&two, "A")
        .unwrap_err()
        .contains("two lasers"));
    for width in ["0", "-0.1", "1.5"] {
        let t = table(&[&format!("1,A,laser,{width},0,0,,0,,0,0,0,0,0")]);
        let e = kerf_from_crib(&t, "A").unwrap_err();
        assert!(e.contains("above 0 and up to 1 mm"), "{width}: {e}");
    }
}

#[test]
fn a_table_with_more_rows_than_the_limit_is_refused() {
    let mut t = String::from("name,kind,diameter_mm\n");
    for _ in 0..=MAX_CRIB_ROWS + 1 {
        t.push_str("a,drill,1\n");
    }
    assert!(laser_tools(&t).unwrap_err().contains("more than"));
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_crib_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_file_over_the_limit_is_refused_before_it_is_read_and_one_that_is_not_text_says_so() {
    let dir = scratch("limit");
    let big = dir.join("big.csv");
    fs::write(&big, vec![b'a'; MAX_CRIB_BYTES as usize + 1]).unwrap();
    let e = kerf_from_file(big.to_str().unwrap(), "x").unwrap_err();
    assert!(e.contains("over the limit"), "{e}");
    let binary = dir.join("bin.csv");
    fs::write(&binary, [0xffu8, 0xfe, 0x00]).unwrap();
    assert!(kerf_from_file(binary.to_str().unwrap(), "x")
        .unwrap_err()
        .contains("not UTF-8"));
    assert!(kerf_from_file(dir.join("none.csv").to_str().unwrap(), "x").is_err());
    let good = dir.join("crib.csv");
    fs::write(&good, table(&CRIB)).unwrap();
    assert_eq!(
        kerf_from_file(good.to_str().unwrap(), "diode 5 w").unwrap(),
        0.12
    );
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
fn a_damaged_table_is_read_without_a_panic_and_a_kerf_that_comes_out_is_sound() {
    let base = table(&CRIB).into_bytes();
    let mut s = Seed(0x5EED_0038);
    for round in 0..400 {
        let mut bytes = base.clone();
        for _ in 0..1 + s.below(8) {
            let at = s.below(bytes.len() as u64) as usize;
            match s.below(4) {
                0 => bytes[at] = s.below(256) as u8,
                1 => {
                    bytes.remove(at);
                }
                2 => bytes.insert(at, b"\",\n'=\r"[s.below(6) as usize]),
                _ => bytes.truncate(at),
            }
            if bytes.is_empty() {
                break;
            }
        }
        let text = String::from_utf8_lossy(&bytes);
        if let Ok(tools) = laser_tools(&text) {
            for t in &tools {
                if let Ok(k) = kerf_from_crib(&text, &t.name) {
                    assert!(k > 0.0 && k <= 1.0, "round {round}: {k}");
                }
            }
        }
    }
}

const PADS: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";

fn run(args: &[&str]) -> (Option<i32>, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn the_command_line_takes_the_kerf_from_the_crib_and_never_from_both_places() {
    let dir = scratch("cli");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, PADS).unwrap();
    let crib = dir.join("crib.csv");
    fs::write(&crib, table(&CRIB)).unwrap();
    let (l, c) = (layer.to_str().unwrap(), crib.to_str().unwrap());
    let (from_crib, by_hand, default) = (dir.join("a.nc"), dir.join("b.nc"), dir.join("c.nc"));
    let (a, b, d) = (
        from_crib.to_str().unwrap(),
        by_hand.to_str().unwrap(),
        default.to_str().unwrap(),
    );
    assert_eq!(
        run(&[
            "convert",
            l,
            a,
            "--thickness",
            "0.1",
            "--crib",
            c,
            "--crib-laser",
            "Fibre laser, 20 W"
        ])
        .0,
        Some(0)
    );
    assert_eq!(
        run(&["convert", l, b, "--thickness", "0.1", "--kerf", "0.08"]).0,
        Some(0)
    );
    assert_eq!(run(&["convert", l, d, "--thickness", "0.1"]).0, Some(0));
    let (fa, fb, fd) = (
        fs::read(&from_crib).unwrap(),
        fs::read(&by_hand).unwrap(),
        fs::read(&default).unwrap(),
    );
    // the source line names the file, so only the cuts are compared: they are the same as with --kerf 0.08 and not the default kerf
    let cuts = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .lines()
            .filter(|l| l.starts_with('G') || l.starts_with('M'))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(cuts(&fa), cuts(&fb));
    assert_ne!(cuts(&fa), cuts(&fd));
    let (code, why) = run(&["check", l, "--crib", c]);
    assert_eq!(code, Some(1));
    assert!(why.contains("go together"), "{why}");
    let (code, why) = run(&[
        "check",
        l,
        "--crib",
        c,
        "--crib-laser",
        "Fibre laser, 20 W",
        "--kerf",
        "0.1",
    ]);
    assert_eq!(code, Some(1));
    assert!(why.contains("give the kerf twice"), "{why}");
    let (code, why) = run(&["check", l, "--crib", c, "--crib-laser", "Drill 0.8"]);
    assert_eq!(code, Some(1));
    assert!(why.contains("no laser called"), "{why}");
    let _ = fs::remove_dir_all(&dir);
}
