//! The window writes the program `convert` writes with the same values (leads, wait of the laser, passes, origin and bed), and both count the leads in the limit of the bed.

use eh3d_stencilcam::lang::{Lang, Msg};
use eh3d_stencilcam::output::{outside_bed_with_leads, FormLeads};
use eh3d_stencilcam::window::{Status, Window};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-stencilcam");

fn sample() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/paste.gtp")
}

fn convert(out: &Path, options: &[&str]) -> Output {
    Command::new(BIN)
        .arg("convert")
        .arg(sample())
        .arg(out)
        .args(options)
        .output()
        .expect("the program runs")
}

fn window(values: &[(&str, &str)]) -> Window {
    let mut w = Window::new(Lang::En);
    assert!(w.open(&sample()), "{:?}", w.status);
    for (key, text) in values {
        let k = w
            .field_index(key)
            .unwrap_or_else(|| panic!("no field {key}"));
        w.set_field(k, (*text).to_string());
    }
    w
}

fn same_program(name: &str, values: &[(&str, &str)], options: &[&str]) -> String {
    let dir = tempfile::tempdir().unwrap();
    let by_command = dir.path().join(format!("command-{name}"));
    let by_window = dir.path().join(format!("window-{name}"));
    let run = convert(&by_command, options);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let mut w = window(values);
    assert!(w.export(&by_window), "{:?}", w.status);
    let text = std::fs::read_to_string(&by_window).unwrap();
    assert_eq!(
        text,
        std::fs::read_to_string(&by_command).unwrap(),
        "{name}"
    );
    text
}

#[test]
fn the_window_writes_the_program_of_the_command_with_nothing_changed() {
    let text = same_program("a.nc", &[], &[]);
    assert!(text.contains("M3 S1000"), "{text}");
    same_program("a.plt", &[], &[]);
}

#[test]
fn the_window_writes_the_program_of_the_command_with_every_value_of_the_cut() {
    let values = [
        ("feed", "450"),
        ("smax", "800"),
        ("lead_in_mm", "0.5"),
        ("lead_out_mm", "0.2"),
        ("lead_in_round_mm", "0.4"),
        ("lead_out_round_mm", "0"),
        ("lead_in_rect_mm", "0.1"),
        ("lead_out_rect_mm", "0.6"),
        ("laser_wait_s", "0.25"),
        ("passes", "3"),
        ("origin_x", "10"),
        ("origin_y", "12"),
        ("bed_width", "400"),
        ("bed_height", "300"),
    ];
    let options = [
        "--feed",
        "450",
        "--smax",
        "800",
        "--lead-in",
        "0.5",
        "--lead-out",
        "0.2",
        "--lead-in-round",
        "0.4",
        "--lead-out-round",
        "0",
        "--lead-in-rect",
        "0.1",
        "--lead-out-rect",
        "0.6",
        "--laser-wait",
        "0.25",
        "--passes",
        "3",
        "--origin",
        "10,12",
        "--bed",
        "400,300",
    ];
    let text = same_program("b.nc", &values, &options);
    assert!(text.contains("G4 P0.25"), "{text}");
    assert_eq!(text.matches("M3 S800").count(), 12, "4 openings, 3 passes");
    same_program("b.plt", &values, &options);
}

#[test]
fn a_lead_outside_the_bed_stops_the_command_and_the_window_but_not_a_plot() {
    let dir = tempfile::tempdir().unwrap();
    // at the origin 0,0 the contours are inside and the lead-ins and lead-outs are not
    let options = ["--origin", "0,0", "--bed", "400,300"];
    let out = dir.path().join("c.nc");
    let run = convert(&out, &options);
    assert!(!run.status.success());
    let said = String::from_utf8_lossy(&run.stderr);
    assert!(said.contains("outside the bed"), "{said}");
    assert!(!out.exists());

    let values = [
        ("origin_x", "0"),
        ("origin_y", "0"),
        ("bed_width", "400"),
        ("bed_height", "300"),
    ];
    let mut w = window(&values);
    assert!(!w.export(&out));
    assert!(
        matches!(w.status, Status::OutsideBed { x, y, .. } if x < 0.0 || y < 0.0),
        "{:?}",
        w.status
    );
    assert!(w.status.is_problem());
    assert!(w
        .status
        .text(Lang::En)
        .contains("outside the bed of 400 x 300 mm"));
    assert!(!out.exists());

    // a plot has no leads, and neither has G-code with its leads at 0
    same_program("c.plt", &values, &options);
    let none = [("lead_in_mm", "0"), ("lead_out_mm", "0")];
    let all: Vec<_> = values.iter().chain(none.iter()).copied().collect();
    same_program(
        "d.nc",
        &all,
        &[
            "--origin",
            "0,0",
            "--bed",
            "400,300",
            "--lead-in",
            "0",
            "--lead-out",
            "0",
        ],
    );
}

#[test]
fn the_bed_counts_where_a_lead_starts_and_ends() {
    let square = vec![vec![
        (1.0, 1.0),
        (2.0, 1.0),
        (2.0, 2.0),
        (1.0, 2.0),
        (1.0, 1.0),
    ]];
    let inside = FormLeads::uniform(1.0, 1.0);
    assert_eq!(
        outside_bed_with_leads(&square, &[], &inside, (3.0, 3.0)),
        None
    );
    // the lead-in starts before the first point, along the first side
    let long_in = FormLeads::uniform(1.5, 0.0);
    assert_eq!(
        outside_bed_with_leads(&square, &[], &long_in, (3.0, 3.0)),
        Some((-0.5, 1.0))
    );
    // the lead-out goes on after the last point, along the last side
    let long_out = FormLeads::uniform(0.0, 1.5);
    assert_eq!(
        outside_bed_with_leads(&square, &[], &long_out, (3.0, 3.0)),
        Some((1.0, -0.5))
    );
    assert_eq!(
        outside_bed_with_leads(&square, &[], &FormLeads::uniform(0.0, 0.0), (2.0, 2.0)),
        None
    );
    assert_eq!(
        outside_bed_with_leads(&[vec![]], &[], &inside, (1.0, 1.0)),
        None
    );
}

#[test]
fn a_value_of_the_cut_that_the_command_refuses_is_refused_by_the_window() {
    let cases = [
        ("lead_in_mm", "6", Msg::LeadIn),
        ("lead_out_mm", "-1", Msg::LeadOut),
        ("lead_in_round_mm", "x", Msg::LeadInRound),
        ("laser_wait_s", "61", Msg::LaserWait),
        ("passes", "0", Msg::Passes),
        ("passes", "11", Msg::Passes),
        ("feed", "0.5", Msg::Feed),
        ("origin_x", "1", Msg::OriginY),
        ("bed_height", "5", Msg::BedWidth),
    ];
    let dir = tempfile::tempdir().unwrap();
    for (key, text, named) in cases {
        let mut w = window(&[(key, text)]);
        assert!(
            matches!(&w.status, Status::BadValue { label, .. } if *label == named),
            "{key}={text}: {:?}",
            w.status
        );
        let out = dir.path().join("bad.nc");
        assert!(!w.export(&out), "{key}={text}");
        assert!(!out.exists());
    }
}

#[test]
fn the_same_opening_twice_is_cut_once_as_the_command_does() {
    let dir = tempfile::tempdir().unwrap();
    let twice = dir.path().join("paste.gtp");
    std::fs::write(
        &twice,
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX20000Y20000D03*\nM02*\n",
    )
    .unwrap();
    let mut w = Window::new(Lang::En);
    assert!(w.open(&twice), "{:?}", w.status);
    assert_eq!((w.openings.len(), w.cuts.len(), w.duplicates), (1, 1, 1));
}
