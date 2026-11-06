<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-STENCILCAM banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-STENCILCAM
<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Solder paste stencils from the paste layer: openings, ratio checks and cutting paths

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust library and a command line program (`eh3d-stencilcam`) with 146 tests that run on a computer: turns the pads of a paste layer into stencil openings (shrunk, rounded, compensated for the laser kerf and checked for paste release) and writes them as G-code or HP-GL. It has a window (built with `--features gui`) that opens a paste layer and draws the pads and the cutting paths, but the window has not been looked at on a screen yet; it has not been tried on a machine and it has not been checked against a collection of real files.

---

> **Safety and security status:** this program does not control any machine and is not a safety system. What it produces must be checked by a person before anything is run on a machine; nothing here has run on real hardware.


## 📋 At a glance

| | |
|---|---|
| Version | `0.0.6` |
| Maturity | Scaffolding |
| Tests | 146 |
| Stack | Rust |
| On real hardware | Not tried: nothing here has run on a machine |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Structure](#-repository-structure) · [Related](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Core | 🟡 | turns the pads of a paste layer into stencil openings (shrunk, rounded, compensated for the laser kerf and checked for paste release) and writes them as G-code or HP-GL; tested on a computer with normal and damaged input, not with a collection of real files. |
| Command line | 🟡 | `eh3d-stencilcam` runs the same functions as the library and prints the result; every error comes back as a message and a non-zero exit code. |
| Tests | 🟡 | 146 tests pass (`cargo test`), with normal cases, edge cases and damaged input. |
| Security | 🟡 | Plain safe Rust (`forbid(unsafe_code)`): nothing panics on the files it is given and a bad value is an error with its reason; no independent review. |
| Window | 🟠 | Written (egui, `--features gui`): open or drop a paste layer, choose the material, edit the parameters, see the pads and the cutting paths (the openings that will not release the paste in red), zoom and pan, export; its logic is tested, but it has not been built or looked at on a screen yet. |
| Real hardware | 🔴 | Not tried: nothing here has been run on a machine. |
| Documentation | 🟡 | README in eight languages, the changelog and the two pictures drawn from `docs/`. |

## 🎯 Overview

* **The openings** (`src/openings.rs`): every pad flashed in the paste layer (rectangular, round or obround, in millimetres whatever the units of the file) is an opening; it is shrunk by the percentage of the material, and the cut line is made smaller by the kerf of the laser so that the hole comes out the size that was asked. A stencil for the bottom side can be mirrored.
* **The ratios** (`src/openings.rs`): the aspect ratio (the small side over the thickness of the sheet, at least 1.5) and the area ratio (the area of the opening over the area of its walls, at least 0.66) say whether the paste will leave the opening; an opening that fails is reported with its position, and `convert` refuses to write until it is looked at (or `--force`).
* **The paths** (`src/output.rs`, `src/profile.rs`): rounded rectangles, circles and obrounds as closed paths in the order of a nearest-first walk; G-code for a laser at constant power with the laser off while travelling, or HP-GL at 40 units to the millimetre; profiles for Mylar, Kapton and steel with their thickness, shrink, corner radius and kerf as starting points.
* **An incomplete stencil is not written:** `convert` refuses to write the file when the layer has pads, tracks or regions that cannot become openings (polygons, macros, regions) unless `--force` is given.
* **The layer is read from its bytes:** `--max-bytes N` refuses a file over the limit (256 MiB by default); text that is not UTF-8 is read as Latin-1 with a warning. The warnings and errors of the reader are printed with their stable code and `--hide CODE[,CODE]` leaves out the notices of those codes; an error of the reader counts as a part of the layer that was lost, so `convert` refuses it unless `--force` is given. The openings come from the conversion of apertures of EH3D-GERBERVIEWER, and a polygon pad is cut as a circle (with a note, so `convert` refuses it too unless `--force` is given). `--arc-tolerance MM` sets how closely arcs are followed, and the refusal of a large file is written in the language of the system.

## 🧩 Role in EH3D

EH3D-STENCILCAM is one program of the EH3D Fabrication Suite. It works as a library and a command line and, built with `--features gui`, as a window in the look the other programs of the suite share. It reads and writes plain files, so it can be used on its own or between the other programs.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-STENCILCAM architecture: who uses it, what it holds and what it depends on" width="100%">
</p>


How the parts of this repository fit together: who uses it, what it holds and what it depends on. The picture is drawn from `docs/architecture.json` and checked against the code.

## ✨ Features

* **A library and a command line:** every command of `eh3d-stencilcam` is a function of the library, so another program of the suite can call it without starting a process.
* **Strict command line:** an option that is written wrong (`--powr`) or a word too many is an error that names it, not something to ignore; the program would otherwise make its output with the default values.
* **Tests with damaged input:** the tests also feed empty files, garbage, huge numbers and cut files, and none of them may panic.
* **No guessing:** a value that does not make sense is an error that says which value and why, never a result that only looks right.
* **No unsafe code:** `#![forbid(unsafe_code)]` in the library and the program.
* **The G-code carries the standard header and a faster order:** the file starts with comment lines for the generator, the units and the layer with its SHA-256, a rectangle with sharp corners has no repeated points, and the order of the cuts is found with a grid, so a hundred thousand openings are ordered in moments.
* **A wait after the laser is switched on:** `--laser-wait SECONDS` writes a `G4` wait after each `M3` so that the laser is at its power before the head moves (0 by default: no wait); the tests read the G-code back with the reader of the suite and find no warning and no cut with the laser off. `--rapid MM_MIN` (or `rapid_mm_min` in `stencil.cfg`, from 100 to 100000, 0 by default: no estimate) is the speed of the moves with the laser off: `convert` then prints the estimated time of the G-code job (cutting, waiting for the laser and moving; the acceleration of the machine is not counted).
* **A straight lead-in and lead-out on every cut:** the G-code can enter each contour along a straight line tangent to its first segment and leave it along the prolongation of its last one (0.3 mm each by default, `--lead-in MM`, `--lead-out MM` or `lead_in_mm` and `lead_out_mm` in `stencil.cfg`; 0 writes none and the program is the one written without leads). The laser stays off while the head travels to the start of the lead-in, is switched on there (and waits, if asked), and is switched off at the end of the lead-out. The outlines are convex and run counter-clockwise, so both leads lie outside the opening, on the sheet that is kept; a lead is not written in HP-GL.

## 🔒 Security & Safety

| Threat | Defense |
|---|---|
| A damaged, truncated or empty file | no panic: the tests cut valid input at every position and feed garbage |
| A number that makes no sense (negative, not finite, absurdly large) | refused with an error that names the value; nothing is rounded into something plausible |
| Unsafe memory use | plain safe Rust with `forbid(unsafe_code)` |

**What is not defended:** a result that was produced cleanly is not a verified result. Nothing here knows the limits of a machine, and the program has had no independent review. Report a vulnerability privately (see [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

The library is `eh3d_stencilcam` (`use eh3d_stencilcam::...`); the command line is `eh3d-stencilcam`. The commands are these; each prints its result on the standard output and reports an error on the standard error with a non-zero exit code. There is no network access.

```text
EH3D-STENCILCAM/
├── src/
│   ├── browse.rs                                the list of files of the window that opens a file
│   ├── cli_args.rs                              the arguments of the command line, the same file in every small utility
│   ├── config.rs                                the settings file: the lead-in and lead-out of the cuts and the laser wait
│   ├── gui.rs                                   the window (egui, with the gui feature): draws the state and passes on what the person does
│   ├── lang.rs                                  the words of the window in the eight languages
│   ├── layer.rs                                 reads a layer from bytes and shows what the reader says by its code
│   ├── lib.rs                                   the library: what every command uses
│   ├── main.rs                                  the command line
│   ├── openings.rs                              a part of the logic
│   ├── output.rs                                a part of the logic
│   ├── profile.rs                               a part of the logic
│   ├── shot.rs                                  the picture mode of the window (--shot): its arguments, read and checked without a screen
│   ├── viewport.rs                              zoom and pan of the picture, in plain numbers
│   └── window.rs                                the state of the window without a screen: layer, material, parameters, cuts, export and status
├── tests/
│   ├── estimate.rs                              tests of the estimated time of a job and of the rapid speed of the settings
│   ├── leads.rs                                 tests of the straight lead-in and lead-out of the cuts and of the settings file
│   ├── reading.rs                               tests of reading files: the size limit, text that is not UTF-8, notices by code
│   ├── readme.rs                                tests that run what the README and the run scripts promise
│   ├── stencil.rs                               tests of the library and the command line
│   ├── strict.rs                                tests of the command line: options and words it does not know
│   └── window_cli.rs                            tests that a build without the gui feature stays a command line
├── examples/
│   └── paste.gtp                                a sample to try the program with
├── docs/
│   ├── architecture.json                        data of the Architecture picture
│   └── structure.json                           data of the Repository Structure picture
├── tools/                                       CI check, README check and the picture tools
├── images/                                      banner and the Architecture and Repository Structure pictures
├── .github/                                     CI and release workflows, issue and pull-request templates
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── Cargo.toml Cargo.lock                        the workspace and its locked dependencies
├── rust-toolchain.toml                          the pinned Rust toolchain
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

## ⚙️ Configuration

A small settings file, `stencil.cfg` (`key = value`, one per line, `#` starts a comment), holds what is worth tuning: `lead_in_mm` and `lead_out_mm`, the length of the straight lead-in before each contour and of the lead-out after it (0.3 mm by default, from 0 to 5; 0 writes none), and `laser_wait_s`, the wait after the laser is switched on (0 by default, up to 60). `eh3d-stencilcam config` prints the file with every default (`examples/stencil.cfg` is that file) and `--config FILE` reads one; `--lead-in MM`, `--lead-out MM` and `--laser-wait SECONDS` give the same settings on the command line and win over the file. A key that is not known, given twice or out of its range is an error that says its line. The rest is given as arguments of the command line (see Interfaces) and every default is written in the help of the command (`eh3d-stencilcam help`). The fixed limits live in the code as named constants and are covered by tests.

## 🚀 Quick start

You need a stable Rust toolchain (1.75 or newer).

```bash
cargo build --release
cargo test
cargo run --release -- check examples/paste.gtp
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/paste.gtp   # a picture of the window
```

To take a picture of the window without opening it by hand, run it with `--shot FILE.png` (also `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`): it opens at that size, saves the PNG, keeps no settings and closes by itself (exit code 0 when the file was written, 1 when it was not, 2 for wrong arguments).

## 🧪 Testing

The suite has 146 tests (`cargo test`, run on Windows and Linux in the CI). They check the normal cases, the edge cases and damaged input: empty files, garbage and cut files never panic. The CI also runs `cargo fmt --check`, `cargo clippy` with warnings as errors and `python -I tools/ci_validate.py`.

## 🏗️ Build & Release

The version lives in `Cargo.toml` and in `eh3d.project.json`; each version has its entry in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` build the program, `test.bat` / `test.sh` run the tests and `run.bat` / `run.sh` run it with a sample. Releases are published on GitHub.

### Downloads

Each release has the program with its window for Windows (64 and 32 bits), Linux (64 bits) and macOS (Apple silicon and Intel), as `<name>-<version>-<system>.zip` (Windows) or `.tar.gz` with the program, the licence, the readme, the icon and the examples. The files are built by GitHub Actions, listed in `SHA256SUMS` and signed with the EH3D key. To check a download, compare its hash with the `.sha256` file next to it (`sha256sum -c`) and check its signature with the `.sig` file next to it (`ssh-keygen -Y verify`).

## 📂 Repository Structure

<p align="center">
  <img src="images/repository-structure.svg" alt="Repository structure of EH3D-STENCILCAM" width="100%">
</p>

<details>
<summary>Text version of the tree</summary>

```text
EH3D-STENCILCAM/
├── src/
│   ├── browse.rs                                the list of files of the window that opens a file
│   ├── cli_args.rs                              the arguments of the command line, the same file in every small utility
│   ├── config.rs                                the settings file: the lead-in and lead-out of the cuts and the laser wait
│   ├── gui.rs                                   the window (egui, with the gui feature): draws the state and passes on what the person does
│   ├── lang.rs                                  the words of the window in the eight languages
│   ├── layer.rs                                 reads a layer from bytes and shows what the reader says by its code
│   ├── lib.rs                                   the library: what every command uses
│   ├── main.rs                                  the command line
│   ├── openings.rs                              a part of the logic
│   ├── output.rs                                a part of the logic
│   ├── profile.rs                               a part of the logic
│   ├── shot.rs                                  the picture mode of the window (--shot): its arguments, read and checked without a screen
│   ├── viewport.rs                              zoom and pan of the picture, in plain numbers
│   └── window.rs                                the state of the window without a screen: layer, material, parameters, cuts, export and status
├── tests/
│   ├── estimate.rs                              tests of the estimated time of a job and of the rapid speed of the settings
│   ├── leads.rs                                 tests of the straight lead-in and lead-out of the cuts and of the settings file
│   ├── reading.rs                               tests of reading files: the size limit, text that is not UTF-8, notices by code
│   ├── readme.rs                                tests that run what the README and the run scripts promise
│   ├── stencil.rs                               tests of the library and the command line
│   ├── strict.rs                                tests of the command line: options and words it does not know
│   └── window_cli.rs                            tests that a build without the gui feature stays a command line
├── examples/
│   └── paste.gtp                                a sample to try the program with
├── docs/
│   ├── architecture.json                        data of the Architecture picture
│   └── structure.json                           data of the Repository Structure picture
├── tools/                                       CI check, README check and the picture tools
├── images/                                      banner and the Architecture and Repository Structure pictures
├── .github/                                     CI and release workflows, issue and pull-request templates
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── Cargo.toml Cargo.lock                        the workspace and its locked dependencies
├── rust-toolchain.toml                          the pinned Rust toolchain
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

</details>

## 🗺️ Roadmap & Known limits

**Known limits today**

* The window has not been looked at on a screen yet, and it does not show the ratios of each opening: `check` does.
* It has not been tried on a machine nor with a collection of real files, and it has had no independent review.
* Macro pads, tracks and regions of the paste layer are not turned into openings (a polygon pad is cut as a circle), the profiles of the materials are starting points, and there is no step-by-step multi-pass cutting for a steel sheet.

**Next**

* Try it with a real paste layer and a laser and read the profiles from the tool crib.
* Look at the window on a screen and finish it: the profiles of the tool crib and a view of the paste ratios of each opening.
* Try it with real files and with the other programs of the suite.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): the readers of Gerber, Excellon and G-code
* [EH3D-PHOENIX-LASER](https://github.com/JuanenRac/EH3D-PHOENIX-LASER): laser engraving and cutting
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): the viewer of Gerber layers
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): the tool crib shared by the CAM programs
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): the checking of G-code before it reaches a machine

## 📚 Documentation

* [Changelog](CHANGELOG.md)
* [Architecture and structure data](docs/architecture.json): the data the two pictures are drawn from.

## 🤝 Contributing & Community

* Before a pull request run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `python -I tools/ci_validate.py`: all must pass. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
* The README exists in eight languages: a change to the text is made in all eight files, and the two pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`.
* Report a vulnerability privately: [SECURITY.md](SECURITY.md). Questions, ideas and reports: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
