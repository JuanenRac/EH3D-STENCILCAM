//! The window reads `stencil.cfg` as `--config` of the command does, warns of the same narrow webs and, for a name that ends in `.dxf`, writes the drawing `convert` writes. Every
//! test runs the command on the same files and compares.

use eh3d_stencilcam::lang::{Lang, Msg};
use eh3d_stencilcam::window::{window_files, Status, Window};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-stencilcam");

/// Two pads 0.1 mm apart, one far away.
const CLOSE_PADS: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X1.0*%\nD10*\nX20000Y20000D03*\nX31000Y20000D03*\nX80000Y20000D03*\nM02*\n";

/// A 2 x 1 pad drawn by a macro with its four corners cut: it is opened only with `region_fit_percent`.
const CUT_CORNERS: &str = "%FSLAX24Y24*%\n%MOMM*%\n%AMSHAPE*\n4,1,8,-0.9,-0.5,0.9,-0.5,1,-0.4,1,0.4,0.9,0.5,-0.9,0.5,-1,0.4,-1,-0.4,-0.9,-0.5,0*%\n%ADD10SHAPE*%\nD10*\nX20000Y30000D03*\nM02*\n";

fn sample() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/paste.gtp")
}

fn file(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path
}

fn command(layer: &Path, out: &Path, options: &[&str]) -> Output {
    Command::new(BIN)
        .arg("convert")
        .arg(layer)
        .arg(out)
        .args(options)
        .output()
        .expect("the program runs")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn set(w: &mut Window, key: &str, text: &str) {
    let k = w
        .field_index(key)
        .unwrap_or_else(|| panic!("no field {key}"));
    w.set_field(k, text.to_string());
}

const EVERY_VALUE: &str = "lead_in_mm = 0.5\nlead_out_mm = 0.2\nlead_in_round_mm = 0.4\nlead_out_round_mm = 0\nlead_in_rect_mm = 0.1\nlead_out_rect_mm = 0.6\nlaser_wait_s = 0.25\nrapid_mm_min = 3000\nfeed_mm_min = 450\nsmax = 800\npasses = 3\nmin_web_mm = 0.5\nmin_aspect = 2\nmin_area = 0.7\nregion_fit_percent = 5\n";

#[test]
fn the_window_writes_what_the_command_writes_with_the_same_settings_file() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = file(dir.path(), "stencil.cfg", EVERY_VALUE);
    let by_command = dir.path().join("command.nc");
    let run = command(
        &sample(),
        &by_command,
        &["--config", cfg.to_str().unwrap(), "--force"],
    );
    assert!(run.status.success(), "{}", said(&run.stderr));
    let expected = fs::read_to_string(&by_command).unwrap();
    // the file before the layer and the layer before the file
    for config_first in [true, false] {
        let mut w = Window::new(Lang::En);
        w.set_force(true);
        let order = if config_first {
            [cfg.clone(), sample()]
        } else {
            [sample(), cfg.clone()]
        };
        for path in &order {
            assert!(w.open(path), "{:?}", w.status);
        }
        let out = dir.path().join(format!("window-{config_first}.nc"));
        assert!(w.export(&out), "{:?}", w.status);
        assert_eq!(fs::read_to_string(&out).unwrap(), expected);
        let time = w.estimate.clone().expect("rapid_mm_min gives the time");
        let told = said(&run.stdout);
        assert!(
            told.contains(&format!("estimated time: {time} (")),
            "{told}"
        );
        let line = w.config_text().unwrap();
        assert!(
            line.contains("stencil.cfg") && line.contains("0.5"),
            "{line}"
        );
    }
}

#[test]
fn a_settings_file_with_a_sheet_gives_the_window_the_stencil_of_the_command() {
    let dir = tempfile::tempdir().unwrap();
    let sheets = file(
        dir.path(),
        "sheets.csv",
        "id,name,kind,material,thickness_mm,reduction_mm,kerf_mm,min_aspect_ratio,min_area_ratio\n1,Thin,sheet,Mylar,0.1,0.05,0.08,0,0\n",
    );
    let cfg = file(
        dir.path(),
        "stencil.cfg",
        &format!("sheet_file = {}\nsheet_name = thin\n", sheets.display()),
    );
    let by_command = dir.path().join("command.nc");
    let run = command(
        &sample(),
        &by_command,
        &["--config", cfg.to_str().unwrap(), "--force"],
    );
    assert!(run.status.success(), "{}", said(&run.stderr));
    let mut w = Window::new(Lang::En);
    w.set_force(true);
    assert!(w.open(&cfg), "{:?}", w.status);
    assert_eq!(w.sheet.as_ref().unwrap().name, "Thin");
    assert!(w.open(&sample()), "{:?}", w.status);
    let out = dir.path().join("window.nc");
    assert!(w.export(&out), "{:?}", w.status);
    assert_eq!(
        fs::read_to_string(&out).unwrap(),
        fs::read_to_string(&by_command).unwrap()
    );
    // neither the table nor the settings file is written over
    for input in [&sheets, &cfg] {
        let before = fs::read_to_string(input).unwrap();
        assert!(!w.export(input));
        assert_eq!(w.status, Status::InputFile);
        assert_eq!(fs::read_to_string(input).unwrap(), before);
    }
}

#[test]
fn a_settings_file_the_command_refuses_is_refused_by_the_window_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    for text in [
        "feed_mm_min = 0\n",
        "nonsense = 1\n",
        "passes = 2\npasses = 3\n",
        "sheet_name = Mylar\n",
    ] {
        let cfg = file(dir.path(), "stencil.cfg", text);
        let run = command(
            &sample(),
            &dir.path().join("no.nc"),
            &["--config", cfg.to_str().unwrap(), "--force"],
        );
        assert_eq!(run.status.code(), Some(1), "{text}");
        let mut w = Window::new(Lang::En);
        assert!(w.open(&sample()));
        let fields = w.fields.clone();
        assert!(!w.load_config(&cfg), "{text}");
        match &w.status {
            Status::Failed(why) => {
                // the window names the keys of the file where the command also names its options
                if text.starts_with("sheet_name") {
                    assert!(why.contains("go together"), "{why}");
                } else {
                    assert!(said(&run.stderr).contains(why.as_str()), "{why}");
                }
            }
            other => panic!("{text}: {other:?}"),
        }
        assert_eq!(w.fields, fields);
        assert!(w.config_file.is_none());
    }
    assert!(!Window::new(Lang::En).load_config(&dir.path().join("missing.cfg")));
}

/// The pairs of the warnings `openings A and B ...` of the command, as (A, B).
fn warned(text: &str) -> Vec<(usize, usize)> {
    text.lines()
        .filter_map(|l| l.strip_prefix("warning: openings "))
        .map(|l| {
            let mut words = l.split_whitespace();
            let first = words.next().unwrap().parse().unwrap();
            let second = words.nth(1).unwrap().parse().unwrap();
            (first, second)
        })
        .collect()
}

#[test]
fn the_window_warns_of_the_narrow_webs_the_command_warns_of() {
    let dir = tempfile::tempdir().unwrap();
    let layer = file(dir.path(), "paste.gtp", CLOSE_PADS);
    let wide = file(dir.path(), "wide.cfg", "min_web_mm = 5\n");
    let off = file(dir.path(), "off.cfg", "min_web_mm = 0\n");
    for (cfg, some) in [(None, false), (Some(&wide), true), (Some(&off), false)] {
        let mut options = vec!["--force"];
        if let Some(c) = cfg {
            options.extend(["--config", c.to_str().unwrap()]);
        }
        let run = command(&layer, &dir.path().join("w.nc"), &options);
        assert!(run.status.success(), "{}", said(&run.stderr));
        let mut w = Window::new(Lang::En);
        if let Some(c) = cfg {
            assert!(w.open(c));
        }
        assert!(w.open(&layer), "{:?}", w.status);
        let pairs: Vec<_> = w.webs.iter().map(|p| (p.first, p.second)).collect();
        assert_eq!(pairs, warned(&said(&run.stderr)));
        if some {
            assert!(!pairs.is_empty());
        }
        assert_eq!(w.web_texts(0).len(), usize::from(!pairs.is_empty()));
        assert_eq!(
            w.web_texts(100).len(),
            if pairs.is_empty() { 0 } else { pairs.len() + 1 }
        );
    }
}

#[test]
fn the_default_narrowest_web_warns_without_a_settings_file_in_every_language() {
    let dir = tempfile::tempdir().unwrap();
    // with no shrink the first two pads are 0.05 mm apart, and the third overlaps the second
    let layer = file(
        dir.path(),
        "paste.gtp",
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X1.0*%\nD10*\nX20000Y20000D03*\nX30500Y20000D03*\nX39500Y20000D03*\nM02*\n",
    );
    let mut w = Window::new(Lang::En);
    set(&mut w, "shrink_percent", "0");
    assert!(w.open(&layer), "{:?}", w.status);
    let texts = w.web_texts(9);
    assert_eq!(texts.len(), 3, "{texts:?}");
    assert!(
        texts[1].starts_with("Openings 1 and 2: 0.050 mm"),
        "{texts:?}"
    );
    assert_eq!(texts[2], "Openings 2 and 3 touch or overlap");
    for lang in Lang::ALL {
        w.lang = lang;
        assert!(w.web_texts(9).iter().all(|t| !t.contains('{')));
    }
}

#[test]
fn region_fit_of_the_settings_file_opens_the_layer_the_command_opens() {
    let dir = tempfile::tempdir().unwrap();
    let layer = file(dir.path(), "paste.gtp", CUT_CORNERS);
    let cfg = file(dir.path(), "stencil.cfg", "region_fit_percent = 5\n");
    // without the file the layer has nothing to open
    assert!(!Window::new(Lang::En).open(&layer));
    let by_command = dir.path().join("command.nc");
    let run = command(&layer, &by_command, &["--config", cfg.to_str().unwrap()]);
    assert!(run.status.success(), "{}", said(&run.stderr));
    let mut w = Window::new(Lang::En);
    assert!(w.open(&cfg));
    assert!(w.open(&layer), "{:?}", w.status);
    assert_eq!(w.layer.as_ref().unwrap().approximated.len(), 1);
    let out = dir.path().join("window.nc");
    assert!(w.export(&out), "{:?}", w.status);
    assert_eq!(
        fs::read_to_string(&out).unwrap(),
        fs::read_to_string(&by_command).unwrap()
    );
    // a file that would leave the open layer with nothing is refused and the old values stay
    let none = file(dir.path(), "none.cfg", "region_fit_percent = 0\n");
    assert!(!w.load_config(&none));
    assert_eq!(w.config.region_fit_percent, 5.0);
    assert_eq!(w.openings.len(), 1);
}

#[test]
fn a_name_that_ends_in_dxf_gets_the_drawing_of_the_command() {
    let dir = tempfile::tempdir().unwrap();
    let by_command = dir.path().join("command.dxf");
    let run = command(&sample(), &by_command, &["--force"]);
    assert!(run.status.success(), "{}", said(&run.stderr));
    let expected = fs::read_to_string(&by_command).unwrap();
    let mut w = Window::new(Lang::En);
    assert!(w.open(&sample()));
    let out = dir.path().join("window.DXF");
    // the sample has an opening that will not release the paste, as in the command
    assert!(!w.export(&out));
    assert!(!out.exists());
    w.set_force(true);
    assert!(w.export(&out), "{:?}", w.status);
    assert_eq!(fs::read_to_string(&out).unwrap(), expected);
    assert!(
        matches!(w.status, Status::Drawn { left_out: 0, .. }),
        "{:?}",
        w.status
    );
    assert!(!w.status.is_problem());

    // a file that exists is never replaced
    fs::write(&out, "mine").unwrap();
    assert!(!w.export(&out));
    assert!(
        matches!(w.status, Status::DrawingExists { .. }),
        "{:?}",
        w.status
    );
    assert_eq!(fs::read_to_string(&out).unwrap(), "mine");

    // a drawing has no origin and no bed
    set(&mut w, "origin_x", "1");
    set(&mut w, "origin_y", "1");
    let other = dir.path().join("placed.dxf");
    let run = command(&sample(), &other, &["--force", "--origin", "1,1"]);
    assert_eq!(run.status.code(), Some(1));
    assert!(!w.export(&other));
    assert_eq!(w.status, Status::DrawingPlacement);
    assert!(!other.exists());
}

#[test]
fn an_obround_opening_stops_the_drawing_of_the_window_unless_force_leaves_it_out() {
    let dir = tempfile::tempdir().unwrap();
    let layer = file(
        dir.path(),
        "paste.gtp",
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10O,1.0X0.5*%\n%ADD11C,0.8*%\nD10*\nX20000Y20000D03*\nD11*\nX80000Y20000D03*\nM02*\n",
    );
    let mut w = Window::new(Lang::En);
    set(&mut w, "thickness_mm", "0.1");
    set(&mut w, "shrink_percent", "0");
    assert!(w.open(&layer));
    let out = dir.path().join("t.dxf");
    assert!(!w.export(&out));
    assert!(
        matches!(&w.status, Status::Failed(e) if e.contains("obround")),
        "{:?}",
        w.status
    );
    assert!(!out.exists());
    w.set_force(true);
    assert!(w.export(&out), "{:?}", w.status);
    assert!(
        w.status
            .text(Lang::En)
            .starts_with("1 openings drawn in t.dxf. 1 obround"),
        "{}",
        w.status.text(Lang::En)
    );
}

#[test]
fn gui_takes_a_layer_and_a_settings_file_and_a_command_is_not_a_window() {
    let args = |list: &[&str]| list.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    let paths = |list: &[&str]| Some(list.iter().map(PathBuf::from).collect::<Vec<_>>());
    assert_eq!(window_files(&args(&[])), paths(&[]));
    assert_eq!(window_files(&args(&["gui"])), paths(&[]));
    assert_eq!(window_files(&args(&["gui", "a.gtp"])), paths(&["a.gtp"]));
    for order in [
        ["gui", "a.gtp", "--config", "s.cfg"],
        ["gui", "--config", "s.cfg", "a.gtp"],
    ] {
        assert_eq!(window_files(&args(&order)), paths(&["s.cfg", "a.gtp"]));
    }
    for wrong in [
        &["gui", "--config"][..],
        &["gui", "a.gtp", "b.gtp"],
        &["gui", "--config", "a.cfg", "--config", "b.cfg"],
        &["gui", "--force"],
        &["convert", "a.gtp", "b.nc"],
        &["check"],
    ] {
        assert_eq!(window_files(&args(wrong)), None, "{wrong:?}");
    }
    let layer = sample();
    assert_eq!(
        window_files(&[layer.to_string_lossy().into_owned()]),
        Some(vec![layer])
    );
    assert!(Lang::ALL
        .iter()
        .all(|l| !Msg::SettingsFile.tr(*l).is_empty()));
}
