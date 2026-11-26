# EH3D-STENCILCAM: status

## Current state

EH3D-STENCILCAM is a working program on a computer: a library (`eh3d_stencilcam`), a command line (`eh3d-stencilcam`) and a window (built with `--features gui`). It turns the example paste layers and drawings into openings and cut files and runs them against a twin of a laser link with a lossy line. The suite of tests runs on Windows and Linux in the CI. The window has not been looked at on a screen (it lists the smallest ratios and those of the weakest openings as text); no stencil has been cut.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 51 requirements: 44 implemented, 5 prototype, 2 planned and 0 validated. By progress: 44 done, 5 partial, 0 pending and 2 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Whether the corner radius and the other starting points of the materials should also be read from the tool crib of the suite (the thickness, the reduction, the kerf and the ratio limits of a sheet are read from it when a sheet is chosen).
* Whether an opening may follow a polygon or a track at an angle instead of being a circle, a rectangle or an obround.
* Whether a steel sheet needs a cutting step by step in several passes.

## What it needs from other repositories

* `EH3D-CORE-LIBS`, `EH3D-GERBERVIEWER` and `EH3D-COMMON`, as sibling folders at the versions named in the workflows.
* A laser, a sheet and a paste layer from a real board, for the only validation that counts.

## Known technical debt and omitted tests

* The profiles of the materials (thickness, shrink, corner radius, kerf) are starting points that were not measured; the figures of a sheet are what the person wrote in the crib.
* The drawing the program writes (version 14 of the format, circles and rectangles) was read back by this program only; no CAD program has opened it.
* The ratios in the window are tested as sentences; how they are drawn has not been looked at on a screen.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* Tests that need a laser or a screen are not written: the register names each as `planned:` with the test that will show it.
