# EH3D-STENCILCAM: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because no stencil has been cut with these files and nothing of this program has run on a real laser.

The register has 39 requirements: 33 implemented, 3 prototype, 3 planned and 0 validated. By progress: 33 done, 3 partial, 0 pending and 3 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

## How to read the register

* **ID:** `STNC-NNN`, never reused and never renumbered; a new capability takes the next number.
* **State:** `implemented` is code that is complete and tested on a computer, `prototype` is part of the capability, `planned` is not written.
* **Progress:** `done`, `partial`, `pending` (can be written without hardware) or `blocked` (waits for hardware or for another program of the suite).
* **Evidence:** `simulator` when the test runs against a simulated laser machine and link or a server in memory, `host-test` when it checks a calculation, a reader or a screen-less piece, `none` when nothing is written yet.
* **Test:** the acceptance criterion: `file::name` is the test `name` of `tests/file.rs` (`src/file::name` is a test inside `src/file.rs`), which must exist. For a capability that is not written the cell says `planned:` and the name of the test that will show it; the day that test exists, the row must change.
* **Limits:** what is not done or not proven; a row that is `partial` or `blocked` always says it.

The check is `tests/reqs.rs`: it reads this table, finds each test and fails when a state, a test or a count does not agree.

## Openings

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| STNC-001 | The pads of the paste layer become openings in millimetres whatever the units of the file. | implemented | done | host-test | `stencil::the_pads_of_the_paste_layer_become_openings_in_millimetres` | - |
| STNC-002 | The opening is shrunk by the percentage of the material and then the kerf is taken off the cut line. | implemented | done | host-test | `stencil::the_opening_is_shrunk_and_then_the_kerf_is_taken_off_the_cut_line` | - |
| STNC-003 | An opening that disappears is an error and a bad profile too. | implemented | done | host-test | `stencil::an_opening_that_disappears_is_an_error_and_a_bad_profile_too` | - |
| STNC-004 | The aspect and area ratios decide whether the paste comes out. | implemented | done | host-test | `stencil::the_ratios_decide_if_the_paste_comes_out` | - |
| STNC-005 | Mirroring flips the layer about its middle for the bottom side. | implemented | done | host-test | `stencil::mirroring_flips_the_layer_about_its_middle` | - |
| STNC-006 | A layer with pads that cannot be opened is not written unless forced. | implemented | done | host-test | `cli::a_layer_with_pads_that_cannot_be_opened_is_not_written_unless_forced` | A track at an angle and a region that is not a rectangle are notes with their reason, not openings. |
| STNC-007 | `convert` writes nothing when an opening will not release the paste unless forced, and never when the kerf eats an opening. | implemented | done | host-test | `cli::convert_writes_nothing_when_the_kerf_eats_an_opening_even_with_force` | - |
| STNC-008 | A circle and a closed rectangle of a DXF drawing become a round and a rectangular opening. | implemented | done | host-test | `dxf::a_circle_is_a_round_opening_and_a_closed_rectangle_a_rectangular_one` | - |
| STNC-009 | The second of two equal openings is left out and the first keeps its place. | implemented | done | host-test | `webs::the_second_of_two_equal_openings_is_left_out_and_the_first_keeps_its_place` | - |
| STNC-010 | The gap of two openings is measured between their circles or their nearest sides. | implemented | done | host-test | `webs::the_gap_of_two_round_openings_is_measured_between_their_circles` | - |
| STNC-011 | The profile of every material is valid and they differ. | implemented | done | host-test | `more::the_starting_profile_of_every_material_is_valid_and_they_differ` | The profiles are starting points that were not measured. |

## The file for the laser

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| STNC-012 | The extension of the output chooses HP-GL or G-code whatever its case. | implemented | done | host-test | `cli::the_extension_of_the_output_chooses_hpgl_or_gcode_whatever_its_case` | - |
| STNC-013 | Two runs write the same bytes. | implemented | done | host-test | `cli::two_runs_write_the_same_bytes_and_mirroring_flips_the_x_of_the_cuts` | - |
| STNC-014 | A file is replaced whole and no temporary file is left, also when the place cannot be written. | implemented | done | host-test | `atomic_write::a_file_is_replaced_whole_and_no_temporary_file_is_left` | - |
| STNC-015 | The G-code starts with the standard header and has no cut with the laser off. | implemented | done | host-test | `generated_gcode::the_command_line_writes_the_header_first_and_the_program_has_no_alert` | Read back with the checker of the suite, not run on a laser. |
| STNC-016 | The laser is switched on at the start of the lead-in and waits there. | implemented | done | host-test | `leads::the_laser_is_switched_on_at_the_start_of_the_lead_in_and_waits_there` | - |
| STNC-017 | A rectangle enters on the line of its first side and a circle has its leads tangent to its chords. | implemented | done | host-test | `leads::a_circle_has_its_leads_tangent_to_its_first_and_last_chords` | - |
| STNC-018 | Each form gets its own leads and the same leads for every form write the program of before. | implemented | done | host-test | `lead_forms::each_path_gets_the_leads_of_its_form` | - |
| STNC-019 | The lower-left corner goes to the origin and a point outside the bed is found. | implemented | done | host-test | `placement::a_point_outside_the_bed_is_found` | - |
| STNC-020 | The time of the cut is estimated only when a rapid speed is given and the file is G-code. | implemented | done | host-test | `estimate::convert_prints_the_estimate_only_when_a_rapid_speed_is_given_and_the_file_is_gcode` | An estimate from the speeds, not measured. |
| STNC-021 | A twin of the laser link runs the job with the laser off at the end, also on a lossy line, and gives up when it is never answered. | prototype | partial | simulator | `src/sim::a_lossy_link_with_resends_still_finishes_the_job_and_the_same_seed_repeats_exactly` | A model made for the tests, not a real laser. |

## Reading and robustness

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| STNC-022 | A Latin-1 file is read and the notice carries its code; a file over the limit is refused before it is read. | implemented | done | host-test | `reading::a_file_over_the_limit_is_refused_before_it_is_read` | - |
| STNC-023 | A notice is hidden by its code, and a misspelt code is an error. | implemented | done | host-test | `reading::a_misspelt_code_is_an_error_because_it_would_hide_nothing` | - |
| STNC-024 | An option written wrong or a word too many is an error that names it. | implemented | done | host-test | `strict::an_option_written_wrong_is_an_error_that_names_it` | - |
| STNC-025 | Damaged layers and huge numbers never make an opening or panic. | implemented | done | host-test | `fuzz::damaged_layers_are_read_without_a_panic` | Seeded generation, not a continuous fuzzer. |
| STNC-026 | The numbers of the command line are checked before anything is written. | implemented | done | host-test | `cli::the_numbers_of_the_command_line_are_checked_before_anything_is_written` | - |

## Server and window

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| STNC-027 | Nothing is sent to the server without an address and a token. | implemented | done | host-test | `src/server::nothing_is_sent_without_a_server_or_a_token` | - |
| STNC-028 | The window makes the openings and the paths when a layer is opened and refuses a value outside the limits. | prototype | partial | host-test | `src/window::opening_a_layer_makes_the_openings_and_the_paths` | The logic is tested without a screen; no person has looked at the drawing. |
| STNC-029 | Every sentence of the window has a text in each of the eight languages. | implemented | done | host-test | `src/lang::every_sentence_has_a_text_in_each_of_the_eight_languages` | - |

## Shapes, limits, the crib and what is not written yet

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| STNC-030 | A macro pad that is a rectangle, a rectangle drawn as a region and a track along an axis of the paste layer become openings. | implemented | done | host-test | `shape_open::a_macro_pad_becomes_an_opening` | A polygon pad is still cut as a circle with a note, and a track at an angle or a region of another shape is a note. |
| STNC-031 | The profiles of the materials are read from the tool crib. | planned | blocked | none | `planned:crib_profiles::the_profile_of_a_material_comes_from_the_crib` | Needs the tool crib program to export sheet materials; it exports lasers, and their beam width gives the kerf (see STNC-038). |
| STNC-032 | The window shows the smallest ratios and the ratios of the weakest openings. | prototype | partial | host-test | `src/window::the_window_shows_the_ratios_of_each_opening` | The sentences are tested without a screen; no person has looked at how they are drawn. |
| STNC-033 | A stencil is cut with the output and paste is printed with it. | planned | blocked | none | `planned:real_laser::a_stencil_cut_with_the_output_prints_paste` | Needs a laser, a sheet and a person. |
| STNC-034 | The window is looked at on a screen and matches the picture the program takes of itself. | planned | blocked | none | `planned:window_screen::the_window_matches_the_reference_picture` | Needs a person at a screen. |
| STNC-035 | The limits of the aspect ratio and of the area ratio are the settings `min_aspect` and `min_area`, and options, with their ranges. | implemented | done | host-test | `ratio_limits::the_limits_are_read_from_the_file_with_their_ranges_and_written_back_by_the_default_file` | - |
| STNC-036 | An opening that will not release the paste is told with its ratio, its limit and the thickest sheet for which it would do. | implemented | done | host-test | `ratio_limits::the_thickest_sheet_that_does_for_an_opening_meets_both_limits_and_a_thicker_one_does_not` | - |
| STNC-037 | A check that did not pass ends with the code 2 and a mistake in what was asked with the code 1. | implemented | done | host-test | `ratio_limits::a_check_that_did_not_pass_ends_with_code_two_and_the_limits_can_be_moved` | - |
| STNC-038 | The kerf is taken from a laser tool of the CSV that the tool crib exports, read with limits, and never from two places. | implemented | done | host-test | `crib_kerf::the_kerf_is_the_beam_width_of_the_laser_that_is_named` | The two programs were never run together; the table is the one the crib program writes. |
| STNC-039 | A region that nearly fills its rectangle is opened as that rectangle only when `region_fit_percent` allows it, and the change is told. | implemented | done | host-test | `shape_open::a_pad_with_cut_corners_needs_the_fit_and_is_then_reported_apart` | The corners are not given back: the opening is the enclosing rectangle. |
