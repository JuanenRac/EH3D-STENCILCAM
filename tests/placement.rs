//! `--origin` and `--bed` place the stencil on the sheet.

use eh3d_stencilcam::output::{move_to_origin, outside_bed};

#[test]
fn the_lower_left_corner_goes_to_the_origin() {
    let mut paths = vec![
        vec![(100.0, -20.0), (110.0, -20.0), (110.0, -10.0)],
        vec![(105.0, -15.0)],
    ];
    move_to_origin(&mut paths, (5.0, 5.0));
    assert_eq!(paths[0][0], (5.0, 5.0));
    assert_eq!(paths[0][2], (15.0, 15.0));
    assert_eq!(paths[1][0], (10.0, 10.0));
}

#[test]
fn nothing_moves_without_points() {
    let mut paths: Vec<Vec<(f64, f64)>> = vec![vec![]];
    move_to_origin(&mut paths, (5.0, 5.0));
    assert!(paths[0].is_empty());
}

#[test]
fn a_point_outside_the_bed_is_found() {
    let inside = vec![vec![(0.0, 0.0), (50.0, 40.0)]];
    assert_eq!(outside_bed(&inside, (50.0, 40.0)), None);
    let negative = vec![vec![(1.0, 1.0), (-0.5, 2.0)]];
    assert_eq!(outside_bed(&negative, (50.0, 40.0)), Some((-0.5, 2.0)));
    let far = vec![vec![(60.0, 1.0)]];
    assert_eq!(outside_bed(&far, (50.0, 40.0)), Some((60.0, 1.0)));
    let not_a_number = vec![vec![(f64::NAN, 1.0)]];
    assert!(outside_bed(&not_a_number, (50.0, 40.0)).is_some());
}
