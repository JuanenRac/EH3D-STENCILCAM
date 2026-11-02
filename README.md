<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-STENCILCAM banner" width="100%">
</p>

# 📖 EH3D-STENCILCAM

<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Solder paste stencils from the paste layer: openings, ratio checks and cutting paths

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust library and a command line program (`eh3d-stencilcam`) with 56 tests that run on a computer: turns the pads of a paste layer into stencil openings (shrunk, rounded, compensated for the laser kerf and checked for paste release) and writes them as G-code or HP-GL. It has no window yet, it has not been tried on a machine and it has not been checked against a collection of real files.

---

> **Safety and security status:** this program does not control any machine and is not a safety system. What it produces must be checked by a person before anything is run on a machine; nothing here has run on real hardware.


## 📋 At a glance

| | |
|---|---|
| Version | `0.0.3` |
| Maturity | Scaffolding |
| Tests | 56 |
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
| Tests | 🟡 | 56 tests pass (`cargo test`), with normal cases, edge cases and damaged input. |
| Security | 🟡 | Plain safe Rust (`forbid(unsafe_code)`): nothing panics on the files it is given and a bad value is an error with its reason; no independent review. |
| Window | 🔴 | Planned: the egui window common to the suite; today there is only the command line. |
| Real hardware | 🔴 | Not tried: nothing here has been run on a machine. |
| Documentation | 🟡 | README in eight languages, the changelog and the two pictures drawn from `docs/`. |

## 🎯 Overview

* **The openings** (`src/openings.rs`): every pad flashed in the paste layer (rectangular, round or obround, in millimetres whatever the units of the file) is an opening; it is shrunk by the percentage of the material, and the cut line is made smaller by the kerf of the laser so that the hole comes out the size that was asked. A stencil for the bottom side can be mirrored.
* **The ratios** (`src/openings.rs`): the aspect ratio (the small side over the thickness of the sheet, at least 1.5) and the area ratio (the area of the opening over the area of its walls, at least 0.66) say whether the paste will leave the opening; an opening that fails is reported with its position, and `convert` refuses to write until it is looked at (or `--force`).
* **The paths** (`src/output.rs`, `src/profile.rs`): rounded rectangles, circles and obrounds as closed paths in the order of a nearest-first walk; G-code for a laser at constant power with the laser off while travelling, or HP-GL at 40 units to the millimetre; profiles for Mylar, Kapton and steel with their thickness, shrink, corner radius and kerf as starting points.
* **An incomplete stencil is not written:** `convert` refuses to write the file when the layer has pads, tracks or regions that cannot become openings (polygons, macros, regions) unless `--force` is given.
* **The layer is read from its bytes:** `--max-bytes N` refuses a file over the limit (256 MiB by default); text that is not UTF-8 is read as Latin-1 with a warning. The warnings and errors of the reader are printed with their stable code and `--hide CODE[,CODE]` leaves out the notices of those codes; an error of the reader counts as a part of the layer that was lost, so `convert` refuses it unless `--force` is given. The openings come from the conversion of apertures of EH3D-GERBERVIEWER, and a polygon pad is cut as a circle (with a note, so `convert` refuses it too unless `--force` is given).

## 🧩 Role in EH3D

EH3D-STENCILCAM is one program of the EH3D Fabrication Suite. Today it works as a library and a command line; its window, in the look the other programs of the suite share, is planned. It reads and writes plain files, so it can be used on its own or between the other programs.

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
│   ├── lib.rs                                   the library: what every command uses
│   ├── main.rs                                  the command line
│   ├── cli_args.rs                              the arguments of the command line, the same file in every small utility
│   ├── layer.rs                                 reads a layer from bytes and shows what the reader says by its code
│   ├── openings.rs                              a part of the logic
│   ├── output.rs                                a part of the logic
│   └── profile.rs                               a part of the logic
├── tests/
│   ├── reading.rs                               tests of reading files: the size limit, text that is not UTF-8, notices by code
│   ├── readme.rs                                tests that run what the README and the run scripts promise
│   ├── stencil.rs                               tests of the library and the command line
│   └── strict.rs                                tests of the command line: options and words it does not know
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
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

## ⚙️ Configuration

There is no configuration file yet: what a person chooses is given as arguments of the command line (see Interfaces) and every default is written in the help of the command (`eh3d-stencilcam help`). The fixed limits live in the code as named constants and are covered by tests.

## 🚀 Quick start

You need a stable Rust toolchain (1.75 or newer).

```bash
cargo build --release
cargo test
cargo run --release -- check examples/paste.gtp
```

## 🧪 Testing

The suite has 56 tests (`cargo test`, run on Windows and Linux in the CI). They check the normal cases, the edge cases and damaged input: empty files, garbage and cut files never panic. The CI also runs `cargo fmt --check`, `cargo clippy` with warnings as errors and `python -I tools/ci_validate.py`.

## 🏗️ Build & Release

The version lives in `Cargo.toml` and in `eh3d.project.json`; each version has its entry in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` build the program, `test.bat` / `test.sh` run the tests and `run.bat` / `run.sh` run it with a sample. Releases are published on GitHub.

## 📂 Repository Structure

<p align="center">
  <img src="images/repository-structure.svg" alt="Repository structure of EH3D-STENCILCAM" width="100%">
</p>

<details>
<summary>Text version of the tree</summary>

```text
EH3D-STENCILCAM/
├── src/
│   ├── lib.rs                                   the library: what every command uses
│   ├── main.rs                                  the command line
│   ├── cli_args.rs                              the arguments of the command line, the same file in every small utility
│   ├── layer.rs                                 reads a layer from bytes and shows what the reader says by its code
│   ├── openings.rs                              a part of the logic
│   ├── output.rs                                a part of the logic
│   └── profile.rs                               a part of the logic
├── tests/
│   ├── reading.rs                               tests of reading files: the size limit, text that is not UTF-8, notices by code
│   ├── readme.rs                                tests that run what the README and the run scripts promise
│   ├── stencil.rs                               tests of the library and the command line
│   └── strict.rs                                tests of the command line: options and words it does not know
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
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

</details>

## 🗺️ Roadmap & Known limits

**Known limits today**

* It has no window yet: only the library and the command line exist.
* It has not been tried on a machine nor with a collection of real files, and it has had no independent review.
* Macro pads, tracks and regions of the paste layer are not turned into openings (a polygon pad is cut as a circle), the profiles of the materials are starting points, and there is no step-by-step multi-pass cutting for a steel sheet.

**Next**

* Try it with a real paste layer and a laser, read the profiles from the tool crib and add the window with the view of the openings.
* Add the window in the look shared by the programs of the suite, in the eight languages.
* Try it with real files and with the other programs of the suite.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): the readers of Gerber, Excellon and G-code
* [EH3D-LASER-CAM](https://github.com/JuanenRac/EH3D-LASER-CAM): laser engraving and cutting
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
