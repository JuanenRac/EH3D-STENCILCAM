# EH3D-STENCILCAM: scope

Status: **host-test**. EH3D-STENCILCAM is a library, a command line and a window that turn the paste layer of a board into the cut paths of a solder-paste stencil, written as G-code for a laser or as an HP-GL plot. It has been tried on a computer only, with the example files and a simulated laser link: no stencil has been cut with its output. This page says what it is for, what it will not do and where it sits among the other programs of the suite.

## Purpose

The CAM program of the suite for solder-paste stencils: it reads the pads flashed in the paste layer (or the circles and rectangles of a DXF drawing), makes each one an opening shrunk by the percentage of the material, takes the kerf of the laser off the cut line, says whether the paste will leave each opening (the aspect ratio and the area ratio), and writes the closed paths in a short order with leads, for a laser at constant power.

## What it does

* Turns every rectangular, round or obround pad of the paste layer into an opening in millimetres, shrunk by the percentage of the material, with the kerf taken off the cut line; the stencil for the bottom side can be mirrored.
* Computes the aspect ratio and the area ratio of each opening and refuses to write while an opening will not release the paste (unless `--force`); an opening the kerf would eat is refused even with `--force`.
* Refuses to write an incomplete stencil: a layer with pads, tracks or regions that cannot become openings (polygons, macros, regions) is told, and written only with `--force`.
* Writes G-code for a laser (laser off while travelling, a wait after the laser is switched on, a standard header, a lead-in and lead-out by form, a start at an origin and a bed that refuses a point outside it) or an HP-GL plot at 40 units to the millimetre, always through a temporary file renamed over the old one.
* Warns of narrow webs between openings, leaves out an opening repeated in the same place and estimates the time of the cut when the rapid speed is given.
* Has profiles for Mylar, Kapton and steel as starting points, and shows all of this in a window (built with `--features gui`) in eight languages.

## What it will not do

* No decision about the limits of a laser machine, its interlocks or its emergency stop (see below).
* No opening from a macro pad, a track or a region of the paste layer yet; a polygon pad is cut as a circle with a note (see `NOT_YET.md`).
* No multi-pass cutting step by step for a steel sheet.
* No claim that a stencil cut with this output prints well: the profiles are starting points and nothing has been cut.
* No claim that anything was proven on a laser: the evidence of every capability is `simulator` or `host-test`.

## Where it sits

* **It reads:** the paste layer (Gerber) or a DXF drawing as plain files, its settings file (`stencil.cfg`) and, when asked, the server of the suite.
* **It writes:** a G-code or HP-GL file (never half written and never over the layer), a report of the openings and, when asked, artifacts on the server.
* **It is read by:** the machine-control programs of the suite through those files, and the server; programs exchange work through files or through the server, never by depending on each other.
* **It depends on:** `EH3D-CORE-LIBS` (the Gerber and G-code readers), `EH3D-GERBERVIEWER` (the conversion of apertures) and `EH3D-COMMON` (the client of the server), built next to it as sibling folders.

## What it never decides

The limits of a laser machine, its interlocks and its emergency stop belong to the machine and to its controller, not to this program. The program for the laser writes the laser off while it travels and refuses an opening it cannot cut well, but a file that passes these checks is not a safe one: laser power, ventilation and eye protection are the person's to settle. Nothing here replaces the emergency stop of a machine or the judgement of the person who stands at it.

The requirements of the program, with the state of each and the test that shows it, are in `REQUIREMENTS.md`; the state in a paragraph is in `STATUS.md`.
