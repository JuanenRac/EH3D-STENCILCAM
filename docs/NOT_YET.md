# EH3D-STENCILCAM: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| Openings that follow a polygon or a track at an angle | An opening is a rectangle, a circle or an obround; a region of another shape is a note, or its enclosing rectangle when `region_fit_percent` allows it | the cut paths can follow any closed outline |
| Profiles of the sheet material read from the tool crib | Only the kerf of a laser tool is read from the crib; the profiles for Mylar, Kapton and steel are starting points in the code | the tool crib program has a kind for sheet materials |
| Cutting a steel sheet in several passes | One pass at constant power is the only cut written | a person cuts steel with a laser and says what it needs |
| The ratios drawn on the openings of the picture | The window lists the smallest ratios and those of the weakest openings as text; the picture marks only the openings that will not release the paste | the window has been looked at on a screen and finished |
| A real paste layer and a real laser | Only the example layers and a twin of the laser link have been used | a person cuts a stencil and prints paste with it |
| Arcs in the cut paths | Round and obround openings are written as short lines, which keeps the file simple and the check exact | a controller needs fewer lines |
