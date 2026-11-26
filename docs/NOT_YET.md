# EH3D-STENCILCAM: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| Openings that follow a polygon or a track at an angle | An opening is a rectangle, a circle or an obround; a region of another shape is a note, or its enclosing rectangle when `region_fit_percent` allows it | the cut paths can follow any closed outline |
| Obround openings in the drawing that is written | The drawing holds circles and rectangles, the same entities the reader of drawings takes; an obround opening is refused, or left out with `--force` | the reader of drawings takes slots or polylines with arcs |
| The drawing in the older version 12 of the format | The light polyline that the reader takes belongs to version 13 and later, so the drawing is written as version 14 | the reader takes the older polyline of version 12 |
| Cutting a steel sheet in several passes | One pass at constant power is the only cut written | a person cuts steel with a laser and says what it needs |
| The ratios drawn on the openings of the picture | The window lists the smallest ratios and those of the weakest openings as text; the picture marks only the openings that will not release the paste | the window has been looked at on a screen and finished |
| A real paste layer and a real laser | Only the example layers and a twin of the laser link have been used | a person cuts a stencil and prints paste with it |
| Arcs in the cut paths | Round and obround openings are written as short lines, which keeps the file simple and the check exact | a controller needs fewer lines |
