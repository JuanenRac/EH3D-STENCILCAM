//! The command line refuses an option written wrong and a word too many instead of ignoring them.

use std::process::Command;

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn an_option_written_wrong_is_an_error_that_names_it() {
    for (cmd, args, named) in [
        ("check", vec!["check", "paste.gbr", "--powr"], "--powr"),
        (
            "convert",
            vec!["convert", "paste.gbr", "out.nc", "--feeed", "5"],
            "--feeed",
        ),
    ] {
        let (ok, e) = run(&args);
        assert!(
            !ok && e.contains(named) && e.contains("unknown option"),
            "{cmd}: {e}"
        );
    }
}

#[test]
fn a_word_too_many_is_an_error_that_names_it() {
    for (cmd, args, named) in [
        (
            "check",
            vec!["check", "paste.gbr", "other.gbr"],
            "other.gbr",
        ),
        (
            "convert",
            vec!["convert", "paste.gbr", "out.nc", "more"],
            "more",
        ),
    ] {
        let (ok, e) = run(&args);
        assert!(
            !ok && e.contains(named) && e.contains("unexpected argument"),
            "{cmd}: {e}"
        );
    }
}

#[test]
fn nothing_is_said_against_the_help_and_the_unknown_commands_still_say_so() {
    let (ok, _) = run(&["help"]);
    assert!(ok);
    let (ok, e) = run(&["nonsense", "--powr", "5"]);
    assert!(!ok && !e.is_empty(), "{e}");
}
