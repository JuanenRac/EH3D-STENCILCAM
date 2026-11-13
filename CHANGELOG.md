# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **The pins follow the new releases:** the CI and the release build take `EH3D-COMMON` at `v0.2.1` and `EH3D-CORE-LIBS` at `v0.0.9`, and the manifest asks for `min_common` 0.2.1. The picture of the window in the README is taken again with the program of today.

## [0.0.7] - DXF input, lead-in by form and speed, passes and laser scale settings

- **Fuzz tests with a fixed seed:** every cut of the example drawing and 500 damaged copies of it, damaged layers and numbers that are huge or not numbers are read without a panic; an entity whose size or place overflows is left out with a note.

- **A drawing (DXF) as input:** a file ending in `.dxf` is read by `eh3d_stencilcam::dxf_in` (circles become round openings, closed rectangles with their sides along the axes rectangular ones, the unit of the drawing is read, any other entity is a note and `convert` refuses to write unless `--force` is given).
- **The speed, the laser scale and the passes are settings:** `feed_mm_min` (600), `smax` (1000) and `passes` (1, up to 10; all the contours are cut once before the next pass) in `stencil.cfg`, and `--feed`, `--smax` and `--passes` on the command line, which win over the file; a value out of its range is refused with the name of the option.

- A screenshot of the window in the README, taken with the published program: the example paste layer with the cutting path and the opening that will not release the paste.
- **The G-code and the HP-GL file are written through a temporary file:** the command line (`convert`) and the window write the program to a file beside the real one and rename it over the real one (`output::write_atomic`), so that a cut or a full disk leaves the old file whole and never a program that stops in the middle of a cut. A failure removes the temporary file and is reported with the code `file_unwritable` and the name of the file. Tests in `tests/atomic_write.rs` and `window.rs`.
- **Lead-in and lead-out by form of the opening:** a round opening and a rectangular one can have their own leads, with `lead_in_round_mm`, `lead_out_round_mm`, `lead_in_rect_mm` and `lead_out_rect_mm` in `stencil.cfg` or `--lead-in-round`, `--lead-out-round`, `--lead-in-rect` and `--lead-out-rect` (0 to 5 mm; the option wins over the file). A length that is not given is the general `lead_in_mm` / `lead_out_mm` (or `--lead-in` / `--lead-out`), so a configuration without the new keys writes the same program as before, and an obround opening always uses the general ones. The estimate of the time (`--rapid`) counts the lead of each form. A bad value is an error that names its key and line or its option; `examples/stencil.cfg` shows the four keys as comments and is the same file as `eh3d-stencilcam config`. In the library, `gcode_with_form_leads` and `estimate_time_with_forms` take a `FormLeads` and the form of each path (`forms`, `ordered_indices`); `gcode_with_leads` and `estimate_time` are unchanged. The rules and worked values are in `docs/leads-by-form.md`.

## [0.0.6] - Estimated time of the job, lead-in and lead-out of the cut and the picture mode

- **An estimate of the time of the job:** with `--rapid MM_MIN` (or `rapid_mm_min` in `stencil.cfg`, from 100 to 100000 mm/min, 0 by default: no estimate and the output is the one it always was) `convert` prints the estimated time of the G-code job after the file is written: the cutting at the feed (leads included), the wait after each `M3` and the moves with the laser off from the origin along the order of the cuts. The acceleration of the machine is not counted. The library has `estimate_time` and `duration_text`; the tests use rectangles worked out by hand, the range of the setting and the command line (a plotter file has no estimate). `examples/stencil.cfg` carries the new key and is the same file as `eh3d-stencilcam config`.
- **A straight lead-in and lead-out on every cut:** each contour of the G-code can be entered along a straight line tangent to its first segment and left along the prolongation of its last one (`gcode_with_leads`; 0.3 mm each by default, from 0 to 5). The laser is off while the head travels to the start of the lead-in, is switched on there (after the `--laser-wait` wait, if any) and is switched off at the end of the lead-out. The outlines are convex and counter-clockwise, so both leads lie outside the opening, on the sheet that is kept. With 0 the program is exactly the one written before, and HP-GL has no leads.
- **A settings file, `stencil.cfg`:** the strict keys `lead_in_mm`, `lead_out_mm` and `laser_wait_s` (comments with `#`; an unknown key, a key given twice or a value out of its range is an error with the number of the line), read with `--config FILE` and printed with their defaults by `eh3d-stencilcam config` (`examples/stencil.cfg`); the new options `--lead-in MM` and `--lead-out MM` and the existing `--laser-wait` win over the file.
- **A picture mode for the window:** `--shot FILE.png` (built with `--features gui`) opens the window at the size of `--size WxH` (320 to 8192, 1180x760 by default), with `--theme forge|dark|light`, `--lang CODE` and the files of `--open FILE`, waits `--wait N` frames (0 to 3600, 6 by default), saves what it drew as a PNG file and closes by itself. An existing file is never replaced unless `--replace` is given, the window keeps no settings in this mode, the exit code is 0 when the file was written, 1 when it was not (also when the picture does not come back within 600 frames) and 2 for wrong arguments, and a build without the window says so. The arguments are read by plain code with its tests.

## [0.0.5] - The window of the stencil program, a laser wait before the cut and the Windows icon

- **Binaries for every system:** `release-binaries.yml` builds the program with its window for Windows (64 and 32 bits), Linux and macOS (Apple silicon and Intel), packs each one with the licence, the readme, the icon and the examples, writes `SHA256SUMS` and a build manifest, attests their provenance, verifies the artifact as a stranger would, prepares a draft release only after the owner approves it (a dry run is the default) and opens an issue `build-failed` when anything does not finish.
- **The dependencies on the shared readers name the version that has what is used:** the G-code crate (and for ISOCAM the Excellon one) is required from 0.0.7, the version that brought the way of writing a number, the header of a program and the units given by the caller.
- **A wait after the laser is switched on:** `--laser-wait SECONDS` writes a `G4` wait after each `M3` (0 by default: none; at most 60), so that the laser is at its power before the head moves.
- **The G-code is read back in the tests:** the cuts, with and without the wait, and the file of the command line are read with the reader of the suite and walked the way the strict mode of the G-code debugger walks them; there is no warning and no cut with the laser off. The header of the command line is compared with the expected text.
- **The Windows executable carries the icon of the program:** the file shows it in the explorer and the taskbar, together with the name, the description and the version, embedded by `build.rs`; other systems are not affected.
- **A window:** built with `--features gui`, `eh3d-stencilcam` with no arguments (or with a single layer file, or `gui [LAYER]`) opens a window in the look of the other programs of the suite (amber on charcoal, the icon of the program). A paste layer is opened from the list of files of the window or dropped on it; the pads and the cutting paths are drawn with zoom (wheel) and pan (drag), and the openings that will not release the paste are in red; the material, thickness, shrink, corner radius, kerf, feed and laser power can be edited; the stencil of the bottom side can be mirrored; a box allows writing although there are problems; and the stencil is exported as laser G-code or HP-GL by the name of the file. The command line works as before, and a build without the feature is only a command line.
- **The window speaks the eight languages:** English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal, chosen from the system and changeable in the window.
- **The logic of the window needs no screen:** `window` (layer, material, parameters, cuts, export, status), `viewport` (zoom and pan), `browse` (the list of files) and `lang` (the words) are plain code with their own tests; `gui` only draws them.
- **The G-code starts with the standard header:** the generator and its version, the units, the layer with its SHA-256 and the tools, and a number that is not finite is refused instead of written.
- **A rectangle with sharp corners has no repeated points:** the laser no longer stops at a corner for nothing.
- **The order of the cuts is found with a grid:** a hundred thousand openings are ordered in moments and the order is the same as before.
- **The application icon and the logos of the program:** `images/brand/` holds the icon (SVG, PNG sizes, `.ico` and `.icns`), the horizontal and vertical logos in a light and a dark version and the social preview image.
- **Shared checks updated:** the validator also compares the badge and the Version row of the README with the manifest, fails when one of its checks cannot run, and the CHANGELOG step of the workflow reads the base branch from the environment.
- **The layer is not overwritten:** `convert` refuses an output file that is the layer itself.
- **Limited reading and arc tolerance:** the layer is refused by its size before it is read into memory (the message comes in the eight languages) and `--arc-tolerance` sets how closely arcs are followed.
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
