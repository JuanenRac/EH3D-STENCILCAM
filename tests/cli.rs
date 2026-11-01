use std::fs;
use std::process::Command;

const WITH_POLYGON: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\n%ADD11P,0.8X6*%\nD10*\nX20000Y20000D03*\nD11*\nX40000Y20000D03*\nM02*\n";

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn a_layer_with_pads_that_cannot_be_opened_is_not_written_unless_forced() {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_cli_{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let layer = dir.join("paste.gtp");
    let out = dir.join("out.nc");
    fs::write(&layer, WITH_POLYGON).unwrap();
    let (layer_s, out_s) = (layer.to_str().unwrap(), out.to_str().unwrap());
    let (ok, why) = run(&["convert", layer_s, out_s, "--thickness", "0.1"]);
    assert!(!ok && why.contains("incomplete"), "{why}");
    assert!(!out.exists(), "nothing is written");
    let (ok, why) = run(&["convert", layer_s, out_s, "--thickness", "0.1", "--force"]);
    assert!(ok, "{why}");
    assert!(out.exists());
    let _ = fs::remove_dir_all(&dir);
}
