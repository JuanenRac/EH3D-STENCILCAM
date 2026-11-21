# EH3D-STENCILCAM: status

## Current state

EH3D-STENCILCAM is a working program on a computer: a library (`eh3d_stencilcam`), a command line (`eh3d-stencilcam`) and a window (built with `--features gui`). It turns the example paste layers and drawings into openings and cut files and runs them against a twin of a laser link with a lossy line. The suite of tests runs on Windows and Linux in the CI. The window has not been looked at on a screen and does not show the ratios of each opening; no stencil has been cut.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 34 requirements: 27 implemented, 2 prototype, 5 planned and 0 validated. By progress: 27 done, 2 partial, 2 pending and 3 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* Whether the profiles of the materials should be read from the tool crib of the suite instead of being starting points in the code.
* How macro pads, tracks and regions of the paste layer should become openings.
* Whether a steel sheet needs a cutting step by step in several passes.

## What it needs from other repositories

* `EH3D-CORE-LIBS`, `EH3D-GERBERVIEWER` and `EH3D-COMMON`, as sibling folders at the versions named in the workflows.
* A laser, a sheet and a paste layer from a real board, for the only validation that counts.

## Known technical debt and omitted tests

* The profiles of the materials (thickness, shrink, corner radius, kerf) are starting points that were not measured.
* The window does not show the ratios of each opening; `check` does.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* Tests that need a laser or a screen are not written: the register names each as `planned:` with the test that will show it.
