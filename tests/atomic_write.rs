//! The program for the laser is written through a temporary file that is renamed over the real one: a file is never left half written, and a failure leaves the old file and no
//! temporary file.

use eh3d_stencilcam::output::write_atomic;
use std::fs;
use std::process::Command;

const TWO_PADS: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("eh3d_stencil_atomic_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_file_is_replaced_whole_and_no_temporary_file_is_left() {
    let dir = scratch("replace");
    let file = dir.join("out.nc");
    write_atomic(&file, "first\n").unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "first\n");
    write_atomic(&file, "second\n").unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "second\n");
    assert!(!dir.join("out.nc.tmp").exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_place_that_cannot_be_written_is_an_error_with_its_code_and_leaves_nothing_behind() {
    let dir = scratch("fail");
    // a folder cannot be replaced by a file
    let folder = dir.join("folder.nc");
    fs::create_dir_all(&folder).unwrap();
    let why = write_atomic(&folder, "x").unwrap_err();
    assert!(
        why.starts_with("file_unwritable: ") && why.contains("folder.nc"),
        "{why}"
    );
    assert!(folder.is_dir());
    assert!(!dir.join("folder.nc.tmp").exists());
    // a folder that is not there
    let why = write_atomic(&dir.join("none").join("out.nc"), "x").unwrap_err();
    assert!(why.starts_with("file_unwritable: "), "{why}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_command_line_replaces_the_old_program_whole_and_fails_without_a_temporary_file() {
    let dir = scratch("cli");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let out = dir.join("out.nc");
    fs::write(&out, "an old program\n").unwrap();
    let run = |target: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
            .args([
                "convert",
                layer.to_str().unwrap(),
                target.to_str().unwrap(),
                "--thickness",
                "0.1",
            ])
            .output()
            .expect("the program runs")
    };
    let done = run(&out);
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    let text = fs::read_to_string(&out).unwrap();
    assert!(
        text.contains("G") && !text.contains("an old program"),
        "{text}"
    );
    assert!(!dir.join("out.nc.tmp").exists());
    // the output is a folder: the run fails with the code and the folder is still there
    let folder = dir.join("folder.nc");
    fs::create_dir_all(&folder).unwrap();
    let failed = run(&folder);
    assert!(!failed.status.success());
    let why = String::from_utf8_lossy(&failed.stderr).into_owned();
    assert!(
        why.contains("file_unwritable: ") && why.contains("folder.nc"),
        "{why}"
    );
    assert!(folder.is_dir());
    assert!(!dir.join("folder.nc.tmp").exists());
    let _ = fs::remove_dir_all(&dir);
}
