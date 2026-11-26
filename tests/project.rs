//! The `.eh3d` project: the paste layers are read from the job, each stencil is kept in the branch of this program only, a job that changes makes them out of date, and the window chooses
//! the layer of the job and the sheet of the table. Every file is made here from invented data.

use eh3d_gerberviewer::eh3dfile::{self as suite, SOURCE_BRANCH};
use eh3d_project::{BranchUpdate, NewProject, OpenOptions, Project, SaveOptions, Writer};
use eh3d_stencilcam::eh3dfile::{self, KIT};
use eh3d_stencilcam::lang::Lang;
use eh3d_stencilcam::window::{Status, Window};
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-stencilcam");
/// Two rectangular pads of 2 x 1.5 mm.
const PASTE: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,2.0X1.5*%\nD10*\nX00000Y00000D03*\nX100000Y00000D03*\nM02*\n";
/// One round pad of 1.5 mm.
const OTHER: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,1.5*%\nD10*\nX50000Y50000D03*\nM02*\n";
const SHEETS: &str = "name,kind,material,thickness_mm,reduction_mm,kerf_mm,min_aspect_ratio,min_area_ratio\nThin,sheet,mylar,0.1,0.02,0.05,0,0\nThick,sheet,mylar,0.2,0.03,0.06,0,0\n";

fn write_branch(path: &Path, branch: &str, files: &[(&str, &[u8])]) {
    let mut update = BranchUpdate::new(branch, "1.0", "0.1.0");
    for (name, bytes) in files {
        update.entries.insert((*name).to_string(), bytes.to_vec());
    }
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .save_branch(&Writer::for_app(branch), update, &SaveOptions::default())
        .unwrap();
}

fn write_job(path: &Path, top: &str) {
    let record = br#"{"format_version":"1.0.0","layers":[
        {"path":"layers/top.gtp","kind":"paste","name":"Top paste"},
        {"path":"layers/bottom.gbp","kind":"paste","name":"Bottom paste"}]}"#;
    write_branch(
        path,
        SOURCE_BRANCH,
        &[
            ("project.json", record),
            ("layers/top.gtp", top.as_bytes()),
            ("layers/bottom.gbp", PASTE.as_bytes()),
        ],
    );
}

fn project(dir: &Path) -> PathBuf {
    let path = dir.join("board.eh3d");
    let new = NewProject {
        id: "test-id".into(),
        name: "Invented board".into(),
        units: "millimetres".into(),
        created: None,
    };
    Project::create(
        &path,
        new,
        &OpenOptions::default(),
        &SaveOptions::default().atomic,
    )
    .unwrap();
    write_branch(&path, "pcb", &[("board.json", b"{}")]);
    write_job(&path, PASTE);
    path
}

fn hash(path: &Path, branch: &str) -> String {
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .branch(branch)
        .unwrap()
        .hash
        .clone()
}

fn run(path: &Path, args: &[&str]) -> std::process::Output {
    Command::new(BIN)
        .arg("project")
        .arg(args[0])
        .arg(path)
        .args(&args[1..])
        .output()
        .expect("the program runs")
}

#[test]
fn the_paste_layers_are_chosen_by_path_or_name_and_name_their_stencils() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let layers = eh3dfile::paste_layers(&path).unwrap();
    assert_eq!(layers.len(), 2);
    let top = eh3dfile::choose(&layers, "top.gtp").unwrap();
    assert_eq!(top, eh3dfile::choose(&layers, "layers/top.gtp").unwrap());
    let why = eh3dfile::choose(&layers, "other.gtp").unwrap_err();
    assert!(why.contains("layers/bottom.gbp"), "{why}");
    assert_eq!(eh3dfile::result_name(top, "nc"), "stencil-top.nc");
    assert_eq!(eh3dfile::result_name(top, "plt"), "stencil-top.plt");
}

#[test]
fn a_job_without_a_paste_layer_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    write_branch(
        &path,
        SOURCE_BRANCH,
        &[("project.json", br#"{"layers":[]}"#)],
    );
    let why = eh3dfile::paste_layers(&path).unwrap_err();
    assert!(why.contains("no paste layer"), "{why}");
}

#[test]
fn convert_keeps_a_stencil_for_each_paste_layer_in_its_own_branch_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let (pcb, job) = (hash(&path, "pcb"), hash(&path, SOURCE_BRANCH));
    let out = run(&path, &["convert"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(hash(&path, "pcb"), pcb);
    assert_eq!(hash(&path, SOURCE_BRANCH), job);
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["stencil-bottom.nc", "stencil-top.nc"]);
    assert_eq!(state.inputs, ["layers/bottom.gbp", "layers/top.gtp"]);
    assert!(!state.out_of_date);
    let code = suite::read_result(&KIT, &path, "stencil-top.nc").unwrap();
    let code = String::from_utf8(code).unwrap();
    assert!(
        code.contains("top.gtp"),
        "the header names the layer: {code}"
    );
    assert_eq!(run(&path, &["info"]).status.code(), Some(0));
}

#[test]
fn one_layer_converted_again_keeps_the_stencils_of_the_others_while_the_job_is_the_same() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    assert!(run(&path, &["convert", "--layer", "top.gtp"])
        .status
        .success());
    assert!(
        run(&path, &["convert", "--layer", "bottom.gbp", "--mirror"])
            .status
            .success()
    );
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["stencil-bottom.nc", "stencil-top.nc"]);
    write_job(&path, OTHER);
    let stale = run(&path, &["info"]);
    assert_eq!(stale.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&stale.stdout).contains("OUT OF DATE"));
    // making one again drops the stencils of the older job
    assert!(run(&path, &["convert", "--layer", "top.gtp", "--force"])
        .status
        .success());
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["stencil-top.nc"]);
    assert!(!state.out_of_date);
}

#[test]
fn a_refused_conversion_leaves_the_project_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let before = std::fs::read(&path).unwrap();
    for args in [
        &["convert", "--thickness", "9"][..],
        &["convert", "--layer", "missing.gtp"],
        &["convert", "--overwrite"],
        &["convert", "--nothing-like-this"],
    ] {
        assert_ne!(run(&path, args).status.code(), Some(0), "{args:?}");
        assert_eq!(std::fs::read(&path).unwrap(), before, "{args:?}");
    }
}

#[test]
fn the_window_opens_the_paste_layers_of_a_project_and_saves_their_stencils() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let mut w = Window::new(Lang::En);
    assert!(!w.save_project());
    assert_eq!(w.status, Status::NoProject);
    assert!(w.open(&path));
    assert_eq!(w.job_layers.len(), 2);
    assert!(matches!(w.status, Status::ProjectOpened { layers: 2, .. }));
    assert_eq!(w.suggested_output().unwrap(), dir.path().join("top.nc"));
    assert!(w.save_project(), "{:?}", w.status);
    assert!(w.choose_job_layer(1));
    assert!(w.save_project(), "{:?}", w.status);
    assert!(!w.choose_job_layer(7));
    let state = suite::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["stencil-bottom.nc", "stencil-top.nc"]);
    write_job(&path, OTHER);
    assert!(w.open(&path));
    assert!(matches!(w.status, Status::ProjectStale { .. }));
}

#[test]
fn a_table_of_sheets_dropped_on_the_window_offers_them_to_choose() {
    let dir = tempfile::tempdir().unwrap();
    let table = dir.path().join("sheets.csv");
    std::fs::write(&table, SHEETS).unwrap();
    let mut w = Window::new(Lang::En);
    assert!(w.open(&table), "{:?}", w.status);
    assert_eq!(w.sheets.len(), 2);
    assert!(matches!(w.status, Status::SheetsRead { count: 2, .. }));
    assert!(w.choose_sheet(Some(1)));
    assert_eq!(w.sheet.as_ref().unwrap().name, "Thick");
    assert!(!w.choose_sheet(Some(9)));
    assert!(w.choose_sheet(None));
    assert!(w.sheet.is_none());
    assert!(!w.load_sheet_table(&dir.path().join("missing.csv")));
    assert_eq!(w.sheets.len(), 2);
}
