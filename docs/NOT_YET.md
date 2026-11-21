# EH3D-STENCILCAM: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| Openings from macro pads, tracks and regions | They cannot become openings with the conversion of apertures of the viewer; a polygon pad is cut as a circle with a note and `convert` refuses it unless forced | the conversion of apertures draws them exactly |
| Profiles read from the tool crib | The profiles for Mylar, Kapton and steel are starting points in the code | the tool crib program exports materials that this one can read |
| Cutting a steel sheet in several passes | One pass at constant power is the only cut written | a person cuts steel with a laser and says what it needs |
| The ratios of each opening in the window | `check` prints them; the window shows the openings and the paths only | the window has been looked at on a screen and finished |
| A real paste layer and a real laser | Only the example layers and a twin of the laser link have been used | a person cuts a stencil and prints paste with it |
| Arcs in the cut paths | Round and obround openings are written as short lines, which keeps the file simple and the check exact | a controller needs fewer lines |
