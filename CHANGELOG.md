# Changelog

All notable changes to this project are documented here.

## [Unreleased]
- **A hidden error still blocks:** `--hide` keeps a notice off the screen but an error of the reader that is hidden still counts, so the stencil of a damaged layer needs `--force`.
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.
- **More tests:** the materials and the limits of their profiles, the reading of a layer with its size limit and its hidden notices, and the G-code and the HP-GL of the cuts.

## [0.0.4] - CI hardening and shared validator

- **Faster, bounded CI:** the Cargo build is cached, a run that a newer push supersedes is cancelled, every job has a time limit, and the Rust jobs live in their own workflow that is skipped when only documents or images change.
- **A fixed Rust toolchain:** `rust-toolchain.toml` names the compiler version (with rustfmt and clippy) that the CI installs, so a new stable release cannot turn the lints red without a change here.
- **Monthly dependency updates:** Dependabot opens one grouped pull request a month for the GitHub Actions.
- **A line for every change:** a pull request that changes code must add a line under `[Unreleased]` of this file (`tools/changelog_pr_check.py`; the label `no-changelog` skips it).
- **Shared checks updated:** the validator of the suite checks the text of the banner against the manifest and reads the files that git tracks.
- **More tests:** the ratios of an opening at their exact limits, the outlines of rectangles, circles and obrounds (size, direction and area), the walk of the paths, the laser being off while it travels, the numbers of the G-code and HP-GL, and every error of the command line, with nothing written after an error.

## [0.0.3] - Openings from the Gerber viewer conversion
- **The CI builds against the tag of each sibling:** the sibling repositories are read from `depends_on` of the manifest (the ones they need in turn included) and checked out at the tag the repository is built from, so a build does not move when a sibling changes; a separate job, `Siblings at main`, builds against the main branch of every sibling to show a break before the sibling is tagged and does not hold a merge.

- **The layer is read from its bytes:** `--max-bytes N` refuses a file over the limit before it is looked at (256 MiB by default), and text that is not UTF-8 is read as Latin-1 with a warning, as the readers of the suite do.
- **The notices of the reader are shown by their stable code:** each warning and error is printed with its code, `--hide CODE[,CODE]` leaves out the notices of those codes (a misspelt code is an error), and an error of the reader counts as a part of the layer that was lost, so `convert` refuses it unless `--force` is given.
- **The openings come from the conversion of apertures of EH3D-GERBERVIEWER** instead of a copy of it (a dependency by path on the sibling repository): the duplicated code is gone. A polygon pad is now cut as a circle, with a note, and is refused without `--force`; before it was left out.
- **One shared helper for the command-line arguments** (`src/cli_args.rs`: flag, switch, num and a strict finish), the same file in every small utility of the suite; the copies this program had behaved the same, so nothing changes for a person.
- **Tests that run what the README and the scripts promise:** the sample of `run.sh`, `run.bat` and the README is the same command, and it is run on the files of `examples/`.
- **The banner of the README** shows the role and the technologies of this program (it showed the text of the shared library).

## [0.0.2] - A strict command line and a refusal of an incomplete stencil

- **Each dependency on a sibling repository by path says the version it needs** (`version = ">=0.0.1", path = "../EH3D-..."`), so Cargo stops at once when the sibling next to this one is older than the program expects.
- **Strict command line:** an option that is written wrong (`--powr`) or a word too many is an error that names it; before, it was ignored and the output was made with the default values.
- **`convert` refuses an incomplete stencil:** when the layer has pads, tracks or regions that cannot become openings (polygons, macros, regions) the file is not written unless `--force` is given.

## [0.0.1] - Solder paste stencils from the paste layer: openings, ratio checks and cutting paths

- **The openings:** the pads of the paste layer in millimetres, shrunk by the percentage of the material, with the cut line reduced by the kerf of the laser; a stencil for the bottom side can be mirrored.
- **The ratios:** the aspect ratio and the area ratio decide whether the paste leaves each opening; an opening that fails is reported and `convert` does not write unless `--force` is given.
- **The paths:** rounded rectangles, circles and obrounds in a nearest-first order, as G-code for a laser or HP-GL for a plotter, with profiles for Mylar, Kapton and steel.
- **The command line** `eh3d-stencilcam` with `convert` and `check`, and a sample paste layer.
- **13 tests** run on a computer, with damaged input.
