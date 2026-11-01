# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **Each dependency on a sibling repository by path says the version it needs** (`version = ">=0.0.1", path = "../EH3D-..."`), so Cargo stops at once when the sibling next to this one is older than the program expects.
- **Strict command line:** an option that is written wrong (`--powr`) or a word too many is an error that names it; before, it was ignored and the output was made with the default values.
- **`convert` refuses an incomplete stencil:** when the layer has pads, tracks or regions that cannot become openings (polygons, macros, regions) the file is not written unless `--force` is given.

## [0.0.1] - Solder paste stencils from the paste layer: openings, ratio checks and cutting paths

- **The openings:** the pads of the paste layer in millimetres, shrunk by the percentage of the material, with the cut line reduced by the kerf of the laser; a stencil for the bottom side can be mirrored.
- **The ratios:** the aspect ratio and the area ratio decide whether the paste leaves each opening; an opening that fails is reported and `convert` does not write unless `--force` is given.
- **The paths:** rounded rectangles, circles and obrounds in a nearest-first order, as G-code for a laser or HP-GL for a plotter, with profiles for Mylar, Kapton and steel.
- **The command line** `eh3d-stencilcam` with `convert` and `check`, and a sample paste layer.
- **13 tests** run on a computer, with damaged input.
