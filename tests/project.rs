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
    assert_eq!(eh3dfile::result_name(&layers, top, "nc"), "stencil-top.nc");
    assert_eq!(
        eh3dfile::result_name(&layers, top, "plt"),
        "stencil-top.plt"
    );
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

/// A job whose paste layers have these paths inside `manufacturing/`.
fn write_layers(path: &Path, files: &[(&str, &str)]) {
    let layers: Vec<String> = files
        .iter()
        .map(|(rel, _)| format!(r#"{{"path":"{rel}","kind":"paste","name":"{rel}"}}"#))
        .collect();
    let record = format!(
        r#"{{"format_version":"1.0.0","layers":[{}]}}"#,
        layers.join(",")
    );
    let mut entries: Vec<(&str, &[u8])> = vec![("project.json", record.as_bytes())];
    entries.extend(files.iter().map(|(rel, text)| (*rel, text.as_bytes())));
    write_branch(path, SOURCE_BRANCH, &entries);
}

#[test]
fn the_top_and_the_bottom_side_with_one_stem_each_keep_their_own_stencil() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    write_layers(
        &path,
        &[("layers/board.gtp", PASTE), ("layers/board.gbp", OTHER)],
    );
    let out = run(&path, &["convert"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let both = ["stencil+board.gbp.nc", "stencil+board.gtp.nc"];
    assert_eq!(suite::state(&KIT, &path).unwrap().results, both);
    for (name, source) in [(both[1], "board.gtp"), (both[0], "board.gbp")] {
        let code = String::from_utf8(suite::read_result(&KIT, &path, name).unwrap()).unwrap();
        assert!(code.contains(source), "{name}: {code}");
    }
    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    assert!(w.choose_job_layer(1));
    assert!(w.save_project(), "{:?}", w.status);
    assert_eq!(suite::state(&KIT, &path).unwrap().results, both);
}

#[test]
fn a_file_name_that_two_paste_layers_share_is_not_chosen_by_guess() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    write_layers(
        &path,
        &[("layers/a/top.gtp", PASTE), ("layers/b/top.gtp", OTHER)],
    );
    let layers = eh3dfile::paste_layers(&path).unwrap();
    let why = eh3dfile::choose(&layers, "top.gtp").unwrap_err();
    assert!(
        why.contains("layers/a/top.gtp") && why.contains("layers/b/top.gtp"),
        "{why}"
    );
    let second = eh3dfile::choose(&layers, "layers/b/top.gtp").unwrap();
    assert_eq!(second.bytes, OTHER.as_bytes());
    assert_eq!(
        eh3dfile::result_name(&layers, second, "nc"),
        "stencil+b+top.gtp.nc"
    );
}

#[test]
fn no_stencil_is_written_over_a_project_or_the_settings_it_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let before = std::fs::read(&path).unwrap();
    let layer = dir.path().join("one.gtp");
    std::fs::write(&layer, PASTE).unwrap();
    let convert = |out: &Path, extra: &[&std::ffi::OsStr]| {
        Command::new(BIN)
            .arg("convert")
            .arg(&layer)
            .arg(out)
            .args(extra)
            .output()
            .expect("the program runs")
    };
    let out = convert(&path, &[]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("project"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let settings = dir.path().join("my.cfg");
    std::fs::write(&settings, "passes = 1\n").unwrap();
    let out = convert(&settings, &["--config".as_ref(), settings.as_os_str()]);
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(&settings).unwrap(), "passes = 1\n");
    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    assert!(!w.export(&path));
    assert_eq!(w.status, Status::SameFile);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn the_window_says_when_keeping_a_stencil_removed_the_signature_of_the_project() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let signer = eh3d_project::Ed25519Signer::from_seed(&[7; 32]);
    Project::open(&path, &OpenOptions::default())
        .unwrap()
        .sign_project(&signer, &SaveOptions::default())
        .unwrap();
    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    assert!(w.notices.is_empty());
    // the window asks first, and nothing is written until the person agrees
    let before = std::fs::read(&path).unwrap();
    assert!(!w.save_project(), "{:?}", w.status);
    let asked: Vec<String> = w
        .pending
        .as_ref()
        .expect("the window asks")
        .iter()
        .map(|s| s.text("en"))
        .collect();
    assert!(asked.iter().any(|n| n.contains("signature")), "{asked:?}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // not agreeing leaves the project as it was, and agreeing afterwards does nothing
    w.cancel_save();
    assert!(w.pending.is_none() && !w.confirm_save());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // opening again forgets the question
    assert!(!w.save_project());
    assert!(w.open(&path), "{:?}", w.status);
    assert!(w.pending.is_none() && !w.confirm_save());
    // agreeing writes and tells what was removed
    assert!(!w.save_project());
    assert!(w.confirm_save(), "{:?}", w.status);
    assert!(w.pending.is_none());
    assert!(
        w.notices.iter().any(|n| n.contains("signature")),
        "{:?}",
        w.notices
    );
    // with no signature left there is nothing to ask
    assert!(w.open(&path), "{:?}", w.status);
    assert!(w.notices.is_empty());
    assert!(w.save_project(), "{:?}", w.status);
    assert!(w.pending.is_none());
}

/// Saves the project the way another program does: the change is made on a copy and the file is replaced with it, so this
/// process has no memory of that write.
fn saved_by_another(path: &Path, text: &str) {
    let copy = path.with_file_name("another.eh3d");
    std::fs::copy(path, &copy).unwrap();
    write_branch(&copy, "pcb", &[("board.json", text.as_bytes())]);
    std::fs::copy(&copy, path).unwrap();
    std::fs::remove_file(&copy).unwrap();
}

#[test]
fn a_project_another_program_saved_is_not_written_over_and_can_be_read_again_or_saved_apart() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    saved_by_another(&path, "{\"made\":1}");
    let bytes = std::fs::read(&path).unwrap();
    assert!(!w.save_project());
    assert!(
        matches!(w.status, Status::ProjectChanged { .. }),
        "{:?}",
        w.status
    );
    assert!(w.status.is_problem());
    assert!(w.status.text(Lang::En).contains("board.eh3d"));
    assert_eq!(w.changed.as_deref(), Some(path.as_path()));
    assert!(w.pending.is_none() && !w.confirm_save());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    // a file that is there is never written over, under another name the work goes beside what the other program saved
    assert!(!w.save_project_as(&path));
    let apart = dir.path().join("mine.eh3d");
    assert!(w.save_project_as(&apart), "{:?}", w.status);
    assert_eq!(w.project.as_deref(), Some(apart.as_path()));
    assert_eq!(hash(&apart, "pcb"), hash(&path, "pcb"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    // what the window wrote itself does not count
    assert!(w.save_project(), "{:?}", w.status);
    // reading it again takes the offer away and lets it be saved
    saved_by_another(&apart, "{\"made\":2}");
    assert!(!w.save_project());
    assert!(w.reopen_project(), "{:?}", w.status);
    assert!(w.changed.is_none() && !w.reopen_project());
    assert!(w.save_project(), "{:?}", w.status);
}

#[test]
fn a_save_by_another_program_while_a_question_waits_refuses_the_agreed_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let signer = eh3d_project::Ed25519Signer::from_seed(&[7; 32]);
    Project::open(&path, &OpenOptions::default())
        .unwrap()
        .sign_project(&signer, &SaveOptions::default())
        .unwrap();
    let mut w = Window::new(Lang::En);
    assert!(w.open(&path), "{:?}", w.status);
    assert!(!w.save_project(), "{:?}", w.status);
    assert!(w.pending.is_some());
    saved_by_another(&path, "{\"made\":3}");
    let bytes = std::fs::read(&path).unwrap();
    assert!(!w.confirm_save());
    assert!(
        matches!(w.status, Status::ProjectChanged { .. }),
        "{:?}",
        w.status
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    w.dismiss_changed();
    assert!(w.changed.is_none());
}

#[test]
fn a_stencil_made_while_another_program_saved_the_project_is_not_kept_over_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let seen = eh3dfile::mark_of(&path).unwrap();
    let layers = eh3dfile::paste_layers(&path).unwrap();
    saved_by_another(&path, "{\"made\":4}");
    let bytes = std::fs::read(&path).unwrap();
    let name = eh3dfile::result_name(&layers, &layers[0], "nc");
    let keep =
        |seen: &str| eh3dfile::save_stencil_since(&path, seen, &layers[0], &name, b"M2\n".to_vec());
    let why = keep(&seen).unwrap_err();
    assert!(why.contains("EH3D-PKG-025"), "{why}");
    // a text that is no mark of an opening shows nothing and is refused too
    assert!(keep("").unwrap_err().contains("EH3D-PKG-025"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    // with a fresh mark it is kept, and what this program wrote itself does not count
    let seen = eh3dfile::mark_of(&path).unwrap();
    keep(&seen).unwrap();
    keep(&seen).unwrap();
}
