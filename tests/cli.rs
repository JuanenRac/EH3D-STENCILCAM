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

// ---- the command line, more edges ----

const TWO_PADS: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\nD10*\nX20000Y20000D03*\nX80000Y20000D03*\nM02*\n";

fn run_full(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-stencilcam"))
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_stencil_more_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_extension_of_the_output_chooses_hpgl_or_gcode_whatever_its_case() {
    let dir = scratch("ext");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let l = layer.to_str().unwrap();
    for (name, starts) in [
        ("a.PLT", "IN;SP1;"),
        ("a.plt", "IN;SP1;"),
        ("a.nc", "G21\n"),
        ("a.txt", "G21\n"),
        ("noext", "G21\n"),
    ] {
        let out = dir.join(name);
        let (c, o, e) = run_full(&["convert", l, out.to_str().unwrap(), "--thickness", "0.1"]);
        assert_eq!(c, Some(0), "{name}: {e}");
        assert!(o.contains("2 openings written"), "{o}");
        let text = fs::read_to_string(&out).unwrap();
        if starts == "G21\n" {
            // G-code carries the standard header first: the generator, its version, the units, the source with its hash and the tools
            assert!(text.starts_with("; generator: eh3d-stencilcam "), "{name}");
            assert!(text.contains("; units: mm (G21)\n"), "{name}");
            assert!(text.contains("; source: paste.gtp sha256:"), "{name}");
            assert!(text.contains("\nG21\nG90\n"), "{name}");
        } else {
            assert!(text.starts_with(starts), "{name}");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn two_runs_write_the_same_bytes_and_mirroring_flips_the_x_of_the_cuts() {
    let dir = scratch("mirror");
    let layer = dir.join("paste.gtp");
    // two pads of different sizes, so that the layer is not its own mirror image
    fs::write(&layer, "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,1.0X0.5*%\n%ADD11R,2.0X0.5*%\nD10*\nX20000Y20000D03*\nD11*\nX80000Y20000D03*\nM02*\n").unwrap();
    let l = layer.to_str().unwrap();
    let (a, b, m) = (dir.join("a.nc"), dir.join("b.nc"), dir.join("m.nc"));
    run_full(&["convert", l, a.to_str().unwrap(), "--thickness", "0.1"]);
    run_full(&["convert", l, b.to_str().unwrap(), "--thickness", "0.1"]);
    run_full(&[
        "convert",
        l,
        m.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--mirror",
    ]);
    assert_eq!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
    // mirroring moves the cuts but keeps the extent of the layer along x
    let extent = |p: &std::path::Path| -> (f64, f64) {
        let t = fs::read_to_string(p).unwrap();
        let xs: Vec<f64> = t
            .lines()
            .filter(|l| l.starts_with("G0 X") || l.starts_with("G1 X"))
            .map(|l| l[4..].split(' ').next().unwrap().parse::<f64>().unwrap())
            .collect();
        (
            xs.iter().cloned().fold(f64::INFINITY, f64::min),
            xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let (ea, em) = (extent(&a), extent(&m));
    assert!(
        (ea.0 - em.0).abs() < 0.1 && (ea.1 - em.1).abs() < 0.1,
        "{ea:?} {em:?}"
    );
    assert_ne!(fs::read(&a).unwrap(), fs::read(&m).unwrap());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn convert_writes_nothing_when_an_opening_will_not_release_the_paste_unless_forced() {
    let dir = scratch("weak");
    let layer = dir.join("paste.gtp");
    fs::write(
        &layer,
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10R,0.3X0.3*%\nD10*\nX20000Y20000D03*\nM02*\n",
    )
    .unwrap();
    let out = dir.join("out.nc");
    let (l, o) = (layer.to_str().unwrap(), out.to_str().unwrap());
    let (c, _, e) = run_full(&["convert", l, o, "--thickness", "0.2", "--kerf", "0"]);
    assert_eq!(c, Some(2), "a check that did not pass is the code 2");
    assert!(
        e.contains("will not release the paste") && e.contains("opening 1"),
        "{e}"
    );
    assert!(!out.exists());
    let (c, _, e) = run_full(&["check", l, "--thickness", "0.2"]);
    assert_eq!(c, Some(2));
    assert!(e.contains("1 openings will not release"), "{e}");
    assert_eq!(
        run_full(&[
            "convert",
            l,
            o,
            "--thickness",
            "0.2",
            "--kerf",
            "0",
            "--force"
        ])
        .0,
        Some(0)
    );
    assert!(out.exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn convert_writes_nothing_when_the_kerf_eats_an_opening_even_with_force() {
    let dir = scratch("kerf");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let out = dir.join("out.nc");
    let (c, _, e) = run_full(&[
        "convert",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--kerf",
        "0.9",
        "--thickness",
        "0.05",
        "--force",
    ]);
    assert_eq!(c, Some(1));
    assert!(e.contains("is gone"), "{e}");
    assert!(!out.exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_numbers_of_the_command_line_are_checked_before_anything_is_written() {
    let dir = scratch("numbers");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let l = layer.to_str().unwrap();
    let out = dir.join("out.nc");
    let o = out.to_str().unwrap();
    for (extra, word) in [
        (vec!["--smax", "0"], "--smax"),
        (vec!["--smax", "100001"], "--smax"),
        (vec!["--feed", "0"], "feed"),
        (vec!["--feed", "30001"], "feed"),
        (vec!["--feed", "fast"], "--feed"),
        (vec!["--material", "wood"], "wood"),
        (vec!["--thickness", "5"], "thickness_mm"),
        (vec!["--shrink", "-1"], "shrink_percent"),
        (vec!["--radius", "nan"], "--radius"),
        (vec!["--kerf"], "needs a value"),
    ] {
        let mut args = vec!["convert", l, o];
        args.extend(extra.iter());
        let (c, _, e) = run_full(&args);
        assert_eq!(c, Some(1), "{extra:?}");
        assert!(e.contains(word), "{extra:?}: {e}");
    }
    assert!(!out.exists());
    // a feed of exactly 30000 and a power of exactly 100000 are the largest accepted
    assert_eq!(
        run_full(&[
            "convert",
            l,
            o,
            "--feed",
            "30000",
            "--smax",
            "100000",
            "--thickness",
            "0.1"
        ])
        .0,
        Some(0)
    );
    assert!(fs::read_to_string(&out).unwrap().contains("M3 S100000"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_layer_and_an_output_in_a_missing_folder_name_their_paths() {
    let dir = scratch("paths");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let (c, _, e) = run_full(&["check", dir.join("none.gtp").to_str().unwrap()]);
    assert_eq!(c, Some(1));
    assert!(e.contains("none.gtp"), "{e}");
    let missing = dir.join("no").join("such").join("out.nc");
    let (c, _, e) = run_full(&[
        "convert",
        layer.to_str().unwrap(),
        missing.to_str().unwrap(),
        "--thickness",
        "0.1",
    ]);
    assert_eq!(c, Some(1));
    assert!(e.contains("out.nc"), "{e}");
    assert_eq!(
        run_full(&["convert", layer.to_str().unwrap()]).0,
        Some(1),
        "an output file is needed"
    );
    assert_eq!(run_full(&["check"]).0, Some(1), "a layer is needed");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn check_reports_the_count_the_thickness_and_the_smallest_ratios() {
    let dir = scratch("check");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let (c, o, e) = run_full(&[
        "check",
        layer.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--shrink",
        "0",
    ]);
    assert_eq!(c, Some(0), "{e}");
    assert!(
        o.contains("2 openings; thickness 0.1 mm, shrink 0 %"),
        "{o}"
    );
    assert!(
        o.contains("smallest aspect ratio 5.00") && o.contains("smallest area ratio 1.67"),
        "{o}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_layer_with_no_pads_and_a_file_that_is_not_gerber_do_not_crash() {
    let dir = scratch("empty");
    let (empty, junk) = (dir.join("empty.gtp"), dir.join("junk.gtp"));
    fs::write(&empty, "%FSLAX24Y24*%\n%MOMM*%\nM02*\n").unwrap();
    fs::write(&junk, [0u8, 159, 146, 150, 255, 254, 1, 2, 3]).unwrap();
    for f in [&empty, &junk] {
        let (c, _, _) = run_full(&["check", f.to_str().unwrap()]);
        assert!(matches!(c, Some(0) | Some(1)), "{f:?}: {c:?}");
        let out = dir.join("o.nc");
        let (c, _, _) = run_full(&["convert", f.to_str().unwrap(), out.to_str().unwrap()]);
        assert!(matches!(c, Some(0) | Some(1)), "{f:?}: {c:?}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_wait_of_the_laser_is_an_option_of_convert_and_its_range_is_checked() {
    let dir = scratch("laser_wait");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let (l, out) = (layer.to_str().unwrap(), dir.join("w.nc"));
    let o = out.to_str().unwrap();
    let (c, _, e) = run_full(&["convert", l, o, "--thickness", "0.1", "--laser-wait", "2"]);
    assert_eq!(c, Some(0), "{e}");
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(text.matches("G4 P2\n").count(), 2, "{text}");
    let none = dir.join("n.nc");
    run_full(&["convert", l, none.to_str().unwrap(), "--thickness", "0.1"]);
    assert!(!fs::read_to_string(&none).unwrap().contains("G4"));
    let (c, _, e) = run_full(&["convert", l, o, "--thickness", "0.1", "--laser-wait", "61"]);
    assert_ne!(c, Some(0));
    assert!(e.contains("--laser-wait"), "{e}");
    let _ = fs::remove_dir_all(&dir);
}
// ---- the server mode ----

/// A server of the computer that answers `n` requests with `status` and `body` and gives back the requests it got.
fn local_server(
    n: usize,
    status: &str,
    body: &str,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let reply = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = std::thread::spawn(move || {
        let mut heads = Vec::new();
        for _ in 0..n {
            let Ok((mut s, _)) = listener.accept() else {
                break;
            };
            let mut buf = vec![0u8; 262_144];
            let mut got = 0;
            loop {
                let k = s.read(&mut buf[got..]).unwrap_or(0);
                if k == 0 {
                    break;
                }
                got += k;
                let text = String::from_utf8_lossy(&buf[..got]).to_string();
                if let Some(split) = text.find("\r\n\r\n") {
                    let want = text[..split]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if got >= split + 4 + want {
                        break;
                    }
                }
            }
            heads.push(String::from_utf8_lossy(&buf[..got]).to_string());
            let _ = s.write_all(reply.as_bytes());
        }
        heads
    });
    (address, handle)
}

const CREATED: &str = "{\"artifact_id\":\"a-7\",\"blob_sha256\":\"ab\",\"size\":9}";

fn link_file(dir: &std::path::Path, address: &str) -> String {
    let cfg = dir.join("link.cfg");
    fs::write(
        &cfg,
        format!("server_address = {address}\nserver_token = program-token\nserver_retries = 0\nserver_timeout_s = 5\n"),
    )
    .unwrap();
    cfg.to_str().unwrap().to_string()
}

#[test]
fn the_gcode_of_convert_is_kept_on_the_server_with_the_values_it_was_cut_with() {
    let dir = scratch("to_server");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let out = dir.join("paste.nc");
    let (address, handle) = local_server(1, "201 Created", CREATED);
    let cfg = link_file(&dir, &address);
    let (c, o, e) = run_full(&[
        "convert",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--config",
        &cfg,
        "--to-server",
    ]);
    assert_eq!(c, Some(0), "{e}");
    assert!(o.contains("2 openings written"), "{o}");
    assert!(o.contains("kept on the server: paste.nc as a-7"), "{o}");
    let heads = handle.join().unwrap();
    assert!(
        heads[0].starts_with("POST /api/v1/artifacts HTTP/1.1"),
        "{}",
        heads[0]
    );
    assert!(heads[0].contains("Bearer program-token"));
    assert!(heads[0].contains("\"name\":\"paste.nc\""), "{}", heads[0]);
    assert!(
        heads[0].contains("\"format\":\"text/x-gcode\""),
        "{}",
        heads[0]
    );
    assert!(heads[0].contains("\"command\":\"convert\""), "{}", heads[0]);
    assert!(
        heads[0].contains("\"thickness_mm\":\"0.1\""),
        "{}",
        heads[0]
    );
    assert!(heads[0].contains("\"openings\":\"2\""), "{}", heads[0]);
    assert!(heads[0].contains("eh3d-stencilcam"), "{}", heads[0]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_plot_is_kept_as_hpgl_and_the_check_as_json() {
    let dir = scratch("plot_and_check");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let out = dir.join("paste.plt");
    let (address, handle) = local_server(2, "201 Created", CREATED);
    let cfg = link_file(&dir, &address);
    let (c, o, e) = run_full(&[
        "convert",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--config",
        &cfg,
        "--to-server",
    ]);
    assert_eq!(c, Some(0), "{e}");
    assert!(o.contains("kept on the server: paste.plt"), "{o}");
    let (c, o, e) = run_full(&[
        "check",
        layer.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--json",
        "--config",
        &cfg,
        "--to-server",
    ]);
    assert_eq!(c, Some(0), "{e}");
    assert!(
        o.contains("kept on the server: paste.check.json as a-7"),
        "{o}"
    );
    let heads = handle.join().unwrap();
    assert!(
        heads[0].contains("\"format\":\"application/vnd.hp-hpgl\""),
        "{}",
        heads[0]
    );
    assert!(
        heads[1].contains("\"name\":\"paste.check.json\""),
        "{}",
        heads[1]
    );
    assert!(
        heads[1].contains("\"format\":\"application/json\""),
        "{}",
        heads[1]
    );
    assert!(heads[1].contains("\"command\":\"check\""), "{}", heads[1]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn to_server_is_refused_before_any_work_without_a_server_and_outside_convert_and_check_json() {
    let dir = scratch("no_server");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let l = layer.to_str().unwrap();
    let out = dir.join("paste.nc");
    let o = out.to_str().unwrap();
    let (c, _, e) = run_full(&["convert", l, o, "--thickness", "0.1", "--to-server"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("--config"), "{e}");
    assert!(!out.exists(), "nothing was written");
    // a settings file with no server is not a server either
    let cfg = dir.join("none.cfg");
    fs::write(&cfg, "feed_mm_min = 600\n").unwrap();
    let (c, _, e) = run_full(&[
        "convert",
        l,
        o,
        "--thickness",
        "0.1",
        "--config",
        cfg.to_str().unwrap(),
        "--to-server",
    ]);
    assert_eq!(c, Some(1));
    assert!(e.contains("server mode is off"), "{e}");
    assert!(!out.exists());
    // the plain check has nothing to keep
    let (c, _, e) = run_full(&["check", l, "--thickness", "0.1", "--to-server"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("convert and of check --json"), "{e}");
    // and the local mode is as it was
    let (c, o2, e) = run_full(&["convert", l, o, "--thickness", "0.1"]);
    assert_eq!(c, Some(0), "{e}");
    assert!(!o2.contains("kept on the server"));
    assert!(out.exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_refusal_of_the_server_is_told_after_the_file_was_written() {
    let dir = scratch("refused");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let out = dir.join("paste.nc");
    let (address, handle) = local_server(
        1,
        "422 Unprocessable Entity",
        "{\"error\":\"bad_artifact\",\"detail\":\"name too long\"}",
    );
    let cfg = link_file(&dir, &address);
    let (c, _, e) = run_full(&[
        "convert",
        layer.to_str().unwrap(),
        out.to_str().unwrap(),
        "--thickness",
        "0.1",
        "--config",
        &cfg,
        "--to-server",
    ]);
    handle.join().unwrap();
    assert_eq!(c, Some(1));
    assert!(
        e.contains("not kept on the server") && e.contains("name too long"),
        "{e}"
    );
    assert!(out.exists(), "the local file stays");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_server_command_shows_the_link_without_the_token_and_what_the_server_answers() {
    let dir = scratch("status");
    let (address, handle) = local_server(
        1,
        "200 OK",
        "{\"name\":\"eh3d-server\",\"version\":\"0.1.8\",\"api_revision\":1,\"status\":\"ok\"}",
    );
    let cfg = link_file(&dir, &address);
    let (c, o, e) = run_full(&["server", "status", "--config", &cfg]);
    handle.join().unwrap();
    assert_eq!(c, Some(0), "{e}");
    assert!(
        o.contains("server_token = ***") && !o.contains("program-token"),
        "{o}"
    );
    assert!(o.contains("eh3d-server 0.1.8 (ok, api 1)"), "{o}");
    let (c, o, _) = run_full(&["server"]);
    assert_eq!(c, Some(0));
    assert!(o.contains("server mode is off"), "{o}");
    let (c, _, e) = run_full(&["server", "restart"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("unknown"), "{e}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_keys_of_the_link_are_read_from_stencil_cfg_with_their_ranges_and_given_once() {
    let dir = scratch("keys");
    let layer = dir.join("paste.gtp");
    fs::write(&layer, TWO_PADS).unwrap();
    let l = layer.to_str().unwrap();
    let cfg = dir.join("stencil.cfg");
    fs::write(&cfg, "server_address = 10.0.0.5:18200\nserver_token = abc\nserver_machine = bench\nserver_timeout_s = 120\nserver_retries = 5\nserver_mdns = off\nserver_max_answer_mib = 8\nfeed_mm_min = 500\n").unwrap();
    let c = cfg.to_str().unwrap();
    let (code, _, e) = run_full(&["check", l, "--thickness", "0.1", "--config", c]);
    assert_eq!(code, Some(0), "{e}");
    for (text, word) in [
        ("server_timeout_s = 0\n", "server_timeout_s"),
        ("server_retries = 6\n", "server_retries"),
        ("server_mdns = maybe\n", "server_mdns"),
        ("server_address = a b\n", "server_address"),
        ("server_retries = 1\nserver_retries = 2\n", "twice"),
    ] {
        fs::write(&cfg, text).unwrap();
        let (code, _, e) = run_full(&["check", l, "--thickness", "0.1", "--config", c]);
        assert_eq!(code, Some(1), "{text}");
        assert!(e.contains(word) && e.contains("line"), "{text}: {e}");
    }
    let _ = fs::remove_dir_all(&dir);
}
