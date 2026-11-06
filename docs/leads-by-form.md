# Lead-in and lead-out by form

The G-code of `eh3d-stencilcam convert` enters each contour along a straight line tangent to its first segment (the lead-in) and leaves it along the prolongation of its last one (the
lead-out). The length of both can depend on the form of the opening.

## Which length an opening gets

| Form of the opening | Lead-in | Lead-out |
|---|---|---|
| Round (a circle) | `lead_in_round_mm` | `lead_out_round_mm` |
| Rectangle | `lead_in_rect_mm` | `lead_out_rect_mm` |
| Obround | `lead_in_mm` | `lead_out_mm` |

A length that is not given is the general one: `lead_in_round_mm` and `lead_in_rect_mm` are `lead_in_mm` when they are missing, and `lead_out_round_mm` and `lead_out_rect_mm` are
`lead_out_mm`. With none of the four keys the program is the one that was written with the general leads for every opening. The form is the one the Gerber aperture has (a circle, a
rectangle or an obround pad); the form does not change when the opening is shrunk, adjusted for the kerf or mirrored.

## Settings

| Key of `stencil.cfg` | Option of `convert` | Range | Default |
|---|---|---|---|
| `lead_in_mm` | `--lead-in` | 0 to 5 mm | 0.3 |
| `lead_out_mm` | `--lead-out` | 0 to 5 mm | 0.3 |
| `lead_in_round_mm` | `--lead-in-round` | 0 to 5 mm | the general lead-in |
| `lead_out_round_mm` | `--lead-out-round` | 0 to 5 mm | the general lead-out |
| `lead_in_rect_mm` | `--lead-in-rect` | 0 to 5 mm | the general lead-in |
| `lead_out_rect_mm` | `--lead-out-rect` | 0 to 5 mm | the general lead-out |

`eh3d-stencilcam config` prints the four new keys as comments (take the `#` off to use one). The option wins over the file for the same key. The general option reaches a form that has no
length of its own, in the command line and in the file: `--lead-in 0.2` with `lead_in_rect_mm = 0.5` in the file gives rectangles 0.5 mm and circles 0.2 mm. A value out of its range is an
error that names the key and its line (file) or the option (command line). The plotter file (HP-GL) has no leads.

## Worked values

A rectangle of 1 x 0.5 mm centred at (2, 2) with sharp corners starts at its upper right corner (2.5, 2.25); its first side runs towards -x and its last one comes up into the start.
With a rectangular lead-in of 0.5 mm and a lead-out of 0.1 mm the head goes to (3, 2.25), the laser is switched on, the lead-in goes to (2.5, 2.25), the contour is cut, and the
lead-out ends at (2.5, 2.35).

A circle of 1 mm across centred at (8, 2) starts at (8.5, 2). Its first chord leaves in the direction (-sin 5.625 degrees, cos 5.625 degrees) = (-0.098017, 0.995185) and its last one comes
up into the start with (0.098017, 0.995185). With a round lead-in of 0.2 mm and a lead-out of 0.45 mm the lead-in starts at (8.5 + 0.2 x 0.098017, 2 - 0.2 x 0.995185) = (8.520, 1.801) and
the lead-out ends at (8.5 + 0.45 x 0.098017, 2 + 0.45 x 0.995185) = (8.544, 2.448).

The estimate of the time (`--rapid`) counts the length of the lead of each opening, so it changes with these settings.
