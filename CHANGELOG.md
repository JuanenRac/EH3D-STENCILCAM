# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.0.1] - Solder paste stencils from the paste layer: openings, ratio checks and cutting paths

- **The openings:** the pads of the paste layer in millimetres, shrunk by the percentage of the material, with the cut line reduced by the kerf of the laser; a stencil for the bottom side can be mirrored.
- **The ratios:** the aspect ratio and the area ratio decide whether the paste leaves each opening; an opening that fails is reported and `convert` does not write unless `--force` is given.
- **The paths:** rounded rectangles, circles and obrounds in a nearest-first order, as G-code for a laser or HP-GL for a plotter, with profiles for Mylar, Kapton and steel.
- **The command line** `eh3d-stencilcam` with `convert` and `check`, and a sample paste layer.
- **13 tests** run on a computer, with damaged input.
